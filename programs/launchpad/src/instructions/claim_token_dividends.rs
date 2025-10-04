use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::sysvar;
use anchor_lang::solana_program::sysvar::instructions::{load_instruction_at_checked, load_current_index_checked};
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::constants::*;
use crate::errors::LaunchpadError;
use crate::state::{GlobalConfig, UserDividendRecord};
use crate::utils::{format_dividend_message, verify_ed25519_ix};
use crate::events::DividendClaimed;

#[derive(Accounts)]
#[instruction(total_dividend_amount: u64)]
pub struct ClaimTokenDividends<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// Global configuration account
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
    )]
    pub global_config: Box<Account<'info, GlobalConfig>>,

    /// Token mint for dividend distribution
    pub token_mint: Account<'info, Mint>,

    /// User's dividend record for this token mint
    #[account(
        init_if_needed,
        payer = user,
        space = UserDividendRecord::SIZE,
        seeds = [USER_DIVIDEND_SEED, token_mint.key().as_ref(), user.key().as_ref()],
        bump,
    )]
    pub user_dividend_record: Box<Account<'info, UserDividendRecord>>,

    /// Vault authority PDA
    /// CHECK: vault authority
    #[account(
        seeds = [VAULT_AUTHORITY.as_ref()],
        bump,
    )]
    pub vault_authority: SystemAccount<'info>,

    /// Token vault for dividend distribution (holds dividend tokens)
    #[account(
        mut,
        seeds = [TOKEN_VAULT, vault_authority.key().as_ref(), token_mint.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = vault_authority,
        token::token_program = token_program
    )]
    pub dividend_vault: Account<'info, TokenAccount>,

    /// User's token account to receive dividends
    #[account(
        mut,
        token::mint = token_mint,
        token::authority = user,
        token::token_program = token_program
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// System variables account for Ed25519 signature verification
    /// CHECK: This is a system-provided instruction system variable
    #[account(address = sysvar::instructions::ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// This method distributes dividends calculated off-chain
/// Sufficient token_mint must be transferred into dividend_vault before distribution
/// And users must require points_signer's signature to claim dividends
pub fn claim_token_dividends(
    ctx: Context<ClaimTokenDividends>,
    total_dividend_amount: u64,
    signature: [u8; 64],
) -> Result<()> {
    let user_dividend_record = &mut ctx.accounts.user_dividend_record;
    let user = &ctx.accounts.user;
    let token_mint = &ctx.accounts.token_mint;
    let clock = Clock::get()?;

    // Initialize dividend record if needed
    if user_dividend_record.user == Pubkey::default() {
        user_dividend_record.user = user.key();
        user_dividend_record.token_mint = token_mint.key();
        user_dividend_record.bump = ctx.bumps.user_dividend_record;
    }

    // Format the message for signature verification
    let message = format_dividend_message(&user.key(), &token_mint.key(), total_dividend_amount);

    // Get the current instruction index and load the previous instruction
    let current_index = load_current_index_checked(&ctx.accounts.instructions_sysvar)?;
    require!(current_index > 0, LaunchpadError::InvalidInstructionIndex);
    let ix: Instruction = load_instruction_at_checked((current_index - 1) as usize, &ctx.accounts.instructions_sysvar)?;

    // Verify dividend signature using points_signer
    verify_ed25519_ix(&ix, &ctx.accounts.global_config.points_signer.to_bytes(), &message, &signature)?;

    // Calculate claimable amount
    let claimable_amount = user_dividend_record.calculate_claimable(total_dividend_amount)?;

    // Check if there's anything to claim
    require!(claimable_amount > 0, LaunchpadError::NoClaimableAmount);

    // Check if vault has sufficient balance
    require!(
        ctx.accounts.dividend_vault.amount >= claimable_amount,
        LaunchpadError::InsufficientVaultBalance
    );

    // Transfer dividends from vault to user
    let vault_authority_seeds = &[
        VAULT_AUTHORITY.as_ref(),
        &[ctx.bumps.vault_authority],
    ];
    let vault_authority_signer = &[&vault_authority_seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.dividend_vault.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            vault_authority_signer,
        ),
        claimable_amount,
    )?;

    // Update user dividend record
    user_dividend_record.update_claim(claimable_amount, clock.unix_timestamp)?;

    // Emit dividend claimed event
    emit!(DividendClaimed {
        user: user.key(),
        token_mint: token_mint.key(),
        claimed_amount: claimable_amount,
        total_claimed: user_dividend_record.total_claimed,
        signed_total_dividend: total_dividend_amount,
        timestamp: clock.unix_timestamp,
    });

    msg!("User {} claimed {} dividend tokens of mint {}",
         user.key(), claimable_amount, token_mint.key());
    msg!("Total claimed by user: {}", user_dividend_record.total_claimed);

    Ok(())
}
