use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
use anchor_lang::solana_program::sysvar;
use anchor_lang::solana_program::sysvar::instructions::{load_instruction_at_checked, load_current_index_checked};
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::constants::*;
use crate::errors::LaunchpadError;
use crate::state::{GlobalConfig, LaunchPool, UserPoint, UserPosition};
use crate::utils::{calculate_sol_allowance, check_launch_active, check_time_window, format_points_message, validate_contribution_amount, validate_points_amount, verify_ed25519_ix};
use crate::events::ParticipationEvent;

#[derive(Accounts)]
#[instruction(points_to_use: u64, total_points: u64)]
pub struct ParticipateWithPoints<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: vault authority
    #[account(
        mut,
        seeds = [VAULT_AUTHORITY.as_ref()],
        bump,
    )]
    pub vault_authority: SystemAccount<'info>,

    /// Global configuration account
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
    )]
    pub global_config: Box<Account<'info, GlobalConfig>>,

    /// CHECK: WSOL mint (verified by address)
    #[account(
        address = anchor_spl::token::spl_token::native_mint::ID
    )]
    pub wsol_mint: Account<'info, Mint>,

    /// Launch pool account
    #[account(
        mut,
        constraint = launch_pool.is_active() @ LaunchpadError::LaunchNotActive,
    )]
    pub launch_pool: Box<Account<'info, LaunchPool>>,

    /// User points account
    #[account(
        init_if_needed,
        payer = user,
        space = UserPoint::SIZE,
        seeds = [USER_POINT_SEED, user.key().as_ref()],
        bump,
    )]
    pub user_point: Box<Account<'info, UserPoint>>,

    /// User position account
    #[account(
        init_if_needed,
        payer = user,
        space = UserPosition::SIZE,
        seeds = [USER_POSITION_SEED, launch_pool.key().as_ref(), user.key().as_ref()],
        bump,
    )]
    pub user_position: Box<Account<'info, UserPosition>>,

    /// Launch pool WSOL vault (for storing raised SOL)
    /// CHECK: PDA account only for storing SOL
    #[account(
        mut,
        seeds = [TOKEN_VAULT, vault_authority.key().as_ref(), wsol_mint.key().as_ref()],
        bump,
        token::mint = wsol_mint,
        token::authority = vault_authority,
        token::token_program = token_program
    )]
    pub wsol_vault: Account<'info, TokenAccount>,

    /// System variables account for Ed25519 signature verification
    /// CHECK: This is a system-provided instruction system variable
    #[account(address = sysvar::instructions::ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn participate_with_points(
    ctx: Context<ParticipateWithPoints>,
    points_to_use: u64,
    total_points: u64,
    signature: [u8; 64],
) -> Result<()> {
    let launch_pool = &mut ctx.accounts.launch_pool;
    let user_point = &mut ctx.accounts.user_point;
    let user_position = &mut ctx.accounts.user_position;
    let user = &ctx.accounts.user;
    let clock = Clock::get()?;

    // Check launch pool status
    check_launch_active(launch_pool)?;
    check_time_window(launch_pool, clock.unix_timestamp)?;

    let message = format_points_message(&user.key(), points_to_use, total_points, &launch_pool.key());

    // Get the current instruction index and load the previous instruction
    let current_index = load_current_index_checked(&ctx.accounts.instructions_sysvar)?;
    require!(current_index > 0, LaunchpadError::InvalidInstructionIndex);
    let ix: Instruction = load_instruction_at_checked((current_index - 1) as usize, &ctx.accounts.instructions_sysvar)?;

    // Verify points signature
    verify_ed25519_ix(&ix, &ctx.accounts.global_config.points_signer.to_bytes(), &message, &signature)?;

    // Calculate the amount of SOL user can invest
    let sol_allowance = calculate_sol_allowance(points_to_use, launch_pool.points_per_sol)?;

    // Verify points amount
    validate_points_amount(points_to_use, total_points, user_point.points_consumed)?;

    // Verify contribution amount
    validate_contribution_amount(sol_allowance, user_position.contributed_sol)?;

    // Transfer SOL to vault
    anchor_lang::system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: user.to_account_info(),
                to: ctx.accounts.wsol_vault.to_account_info(),
            },
        ),
        sol_allowance,
    )?;
    token::sync_native(CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        token::SyncNative {
            account: ctx.accounts.wsol_vault.to_account_info(),
        },
    ))?;

    // 更新发射池状态
    launch_pool.update_raised_amount(sol_allowance)?;
    launch_pool.total_points_consumed = launch_pool.total_points_consumed
        .checked_add(points_to_use)
        .ok_or(LaunchpadError::MathOverflow)?;

    // 更新参与人数
    let is_first_participation = user_position.contributed_sol == 0;
    if is_first_participation {
        launch_pool.participants_count = launch_pool.participants_count
            .checked_add(1)
            .ok_or(LaunchpadError::MathOverflow)?;
    }

    // 更新用户持仓
    if user_position.user == Pubkey::default() {
        user_position.user = user.key();
        user_position.pool = launch_pool.key();
        user_position.bump = ctx.bumps.user_position;
    }

    user_position.update_participation(
        sol_allowance,
        points_to_use,
        clock.unix_timestamp,
    )?;

    user_point.points_consumed += points_to_use;

    // Emit participation event
    emit!(ParticipationEvent {
        pool: launch_pool.key(),
        user: user.key(),
        sol_amount: sol_allowance,
        points_used: points_to_use,
        total_contribution: user_position.contributed_sol,
        pool_raised_total: launch_pool.raised_sol,
        is_first_participation,
        participants_count: launch_pool.participants_count,
        timestamp: clock.unix_timestamp,
    });

    msg!("User {} participated with {} points", user.key(), points_to_use);
    msg!("SOL contributed: {}", sol_allowance);
    msg!("Total raised: {} / {} SOL",
        launch_pool.raised_sol / LAMPORTS_PER_SOL,
        launch_pool.target_sol / LAMPORTS_PER_SOL
    );

    Ok(())
}
