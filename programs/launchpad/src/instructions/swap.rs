use crate::{constants::GLOBAL_CONFIG_SEED, dlmm::{self, types::RemainingAccountsInfo}, events::SwapFeeCharged, state::GlobalConfig};
use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, TokenAccount, Transfer}
};

#[derive(Accounts)]
pub struct DlmmSwap<'info> {
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
    )]
    pub global_config: Box<Account<'info, GlobalConfig>>,


    #[account(
        mut,
        constraint = admin_fee_token_in.owner == global_config.admin,
        constraint = admin_fee_token_in.mint == anchor_spl::token::spl_token::native_mint::ID
    )]
    pub admin_fee_token_in: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = lb_pair.key() == global_config.lb_pair @ crate::errors::LaunchpadError::InvalidLbPair
    )]

    /// CHECK: The pool account (must match global config)
    pub lb_pair: UncheckedAccount<'info>,

    /// CHECK: Bin array extension account of the pool
    pub bin_array_bitmap_extension: Option<UncheckedAccount<'info>>,

    #[account(mut)]
    /// CHECK: Reserve account of token X
    pub reserve_x: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Reserve account of token Y
    pub reserve_y: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = user_token_in.mint == anchor_spl::token::spl_token::native_mint::ID @ crate::errors::LaunchpadError::InvalidTokenMint
    )]
    /// User token account to sell token (must be WSOL)
    pub user_token_in: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    /// User token account to buy token
    pub user_token_out: Box<Account<'info, TokenAccount>>,

    /// CHECK: Mint account of token X
    pub token_x_mint: UncheckedAccount<'info>,
    /// CHECK: Mint account of token Y
    pub token_y_mint: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Oracle account of the pool
    pub oracle: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Referral fee account
    pub host_fee_in: Option<UncheckedAccount<'info>>,

    /// CHECK: User who's executing the swap
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(address = dlmm::ID)]
    /// CHECK: DLMM program
    pub dlmm_program: UncheckedAccount<'info>,

    /// CHECK: DLMM program event authority for event CPI
    pub event_authority: UncheckedAccount<'info>,

    /// CHECK: memo_program
    pub memo_program: UncheckedAccount<'info>,

    /// CHECK: Token program of mint X
    pub token_x_program: UncheckedAccount<'info>,
    /// CHECK: Token program of mint Y
    pub token_y_program: UncheckedAccount<'info>,
    // Bin arrays need to be passed using remaining accounts
    pub system_program: Program<'info, System>,
}

/// Executes a DLMM swap
///
/// # Arguments
///
/// * `ctx` - The context containing accounts and programs.
/// * `amount_in` - The amount of input tokens to be swapped.
/// * `min_amount_out` - The minimum amount of output tokens expected a.k.a slippage
///
/// # Returns
///
/// Returns a `Result` indicating success or failure.
pub fn handle_dlmm_swap<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, DlmmSwap<'info>>,
    amount_in: u64,
    min_amount_out: u64,
    remaining_accounts_info: RemainingAccountsInfo
) -> Result<()> {
    // Calculate 0.05% fee from input tokens (5 basis points)
    let fee_amount = amount_in
        .checked_mul(5)
        .and_then(|v| v.checked_div(10000))
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Calculate actual amount to swap after deducting fee
    let actual_swap_amount = amount_in
        .checked_sub(fee_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Transfer fee from user's input token account to admin fee account
    if fee_amount > 0 {
        // Use token_x_program since user_token_in is always WSOL
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_x_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_in.to_account_info(),
                    to: ctx.accounts.admin_fee_token_in.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            fee_amount,
        )?;
    }

    // Record user's output token balance before swap
    let balance_before = ctx.accounts.user_token_out.amount;

    // Execute direct DLMM swap with actual swap amount (after fee deduction)
    let accounts = dlmm::cpi::accounts::Swap2 {
        lb_pair: ctx.accounts.lb_pair.to_account_info(),
        bin_array_bitmap_extension: ctx
            .accounts
            .bin_array_bitmap_extension
            .as_ref()
            .map(|account| account.to_account_info()),
        reserve_x: ctx.accounts.reserve_x.to_account_info(),
        reserve_y: ctx.accounts.reserve_y.to_account_info(),
        user_token_in: ctx.accounts.user_token_in.to_account_info(),
        user_token_out: ctx.accounts.user_token_out.to_account_info(),
        token_x_mint: ctx.accounts.token_x_mint.to_account_info(),
        token_y_mint: ctx.accounts.token_y_mint.to_account_info(),
        oracle: ctx.accounts.oracle.to_account_info(),
        host_fee_in: ctx
            .accounts
            .host_fee_in
            .as_ref()
            .map(|account| account.to_account_info()),
        user: ctx.accounts.user.to_account_info(),
        token_x_program: ctx.accounts.token_x_program.to_account_info(),
        token_y_program: ctx.accounts.token_y_program.to_account_info(),
        event_authority: ctx.accounts.event_authority.to_account_info(),
        memo_program: ctx.accounts.memo_program.to_account_info(),
        program: ctx.accounts.dlmm_program.to_account_info(),
    };

    // Direct CPI call without signer_seeds - user signs for themselves
    let cpi_context = CpiContext::new(ctx.accounts.dlmm_program.to_account_info(), accounts)
        .with_remaining_accounts(ctx.remaining_accounts.to_vec());
    dlmm::cpi::swap2(cpi_context, actual_swap_amount, min_amount_out, remaining_accounts_info)?;

    // Reload user's output token account to get updated balance
    ctx.accounts.user_token_out.reload()?;

    // Calculate the amount of tokens received from swap
    let balance_after = ctx.accounts.user_token_out.amount;
    let output_amount = balance_after
        .checked_sub(balance_before)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Emit swap fee event
    emit!(SwapFeeCharged {
        user: ctx.accounts.user.key(),
        input_token_mint: ctx.accounts.user_token_in.mint,
        output_token_mint: ctx.accounts.user_token_out.mint,
        amount_in,
        fee_amount,
        actual_swap_amount: actual_swap_amount, // Amount actually swapped after fee deduction
        amount_out: output_amount,
        fee_percentage: 5, // 0.05% represented as basis points
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())

}
