use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::const_pda::const_authority::VAULT_BUMP;
use crate::constants::{TOKEN_VAULT, VAULT_AUTHORITY};
use crate::errors::LaunchpadError;
use crate::state::{LaunchPool, LaunchStatus};
use crate::events::CreatorTokensClaimed;

#[derive(Accounts)]
pub struct ClaimCreatorTokens<'info> {
    /// Creator account, must be the project creator
    #[account(
        mut,
        constraint = creator.key() == launch_pool.creator @ LaunchpadError::NotCreator
    )]
    pub creator: Signer<'info>,

    /// vault authority
    #[account(
        mut,
        seeds = [VAULT_AUTHORITY.as_ref()],
        bump,
    )]
    pub vault_authority: SystemAccount<'info>,

    /// Launch pool account, must be migrated
    #[account(
        mut,
        constraint = launch_pool.status == LaunchStatus::Migrated || launch_pool.status == LaunchStatus::Failed @ LaunchpadError::InvalidStatus,
    )]
    pub launch_pool: Box<Account<'info, LaunchPool>>,

    /// Launch pool token vault
    #[account(
        mut,
        seeds = [TOKEN_VAULT, vault_authority.key().as_ref(), launch_pool.token_mint.as_ref()],
        bump,
        token::mint = launch_pool.token_mint,
        token::authority = vault_authority,
        address = launch_pool.token_vault,
        constraint = launch_pool.token_vault == pool_token_vault.key() @ LaunchpadError::InvalidTokenVault
    )]
    pub pool_token_vault: Box<Account<'info, TokenAccount>>,

    /// Creator token receiving account
    #[account(
        mut,
        token::mint = launch_pool.token_mint,
        token::authority = creator,
    )]
    pub creator_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

/// Creator claim tokens (supports batch claiming)
pub fn claim_creator_tokens(ctx: Context<ClaimCreatorTokens>) -> Result<()> {
    let launch_pool = &mut ctx.accounts.launch_pool;
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Calculate current new claimable amount (already automatically deducts claimed amount)
    let claimable_amount = launch_pool.calculate_creator_claimable_amount(current_time);

    // Verify if there are claimable tokens
    require!(claimable_amount > 0, LaunchpadError::NothingToClaim);

    // Verify if token vault has sufficient balance
    require!(
        ctx.accounts.pool_token_vault.amount >= claimable_amount,
        LaunchpadError::InsufficientLiquidity
    );

    msg!("Creator claiming {} tokens", claimable_amount);
    msg!("Total claimed so far: {} tokens", launch_pool.creator_claimed_tokens);

    // Execute token transfer
    let signer_seeds: &[&[&[u8]]] = &[&[VAULT_AUTHORITY, &[VAULT_BUMP]]];
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_token_vault.to_account_info(),
                to: ctx.accounts.creator_token_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        claimable_amount,
    )?;

    // Update claimed amount
    launch_pool.creator_claimed_tokens = launch_pool.creator_claimed_tokens
        .checked_add(claimable_amount)
        .ok_or(LaunchpadError::MathOverflow)?;

    // Calculate remaining claimable amount
    let remaining_claimable = launch_pool.creator_allocation
        .saturating_sub(launch_pool.creator_claimed_tokens);
    let fully_unlocked = remaining_claimable == 0;

    // Emit creator tokens claimed event
    emit!(CreatorTokensClaimed {
        pool: launch_pool.key(),
        creator: ctx.accounts.creator.key(),
        token_mint: launch_pool.token_mint,
        claimed_amount: claimable_amount,
        total_claimed: launch_pool.creator_claimed_tokens,
        total_allocation: launch_pool.creator_allocation,
        remaining_claimable,
        fully_unlocked,
        timestamp: current_time,
    });

    msg!("Creator tokens claimed successfully");
    msg!("Claimed amount: {} tokens", claimable_amount);
    msg!("Total claimed: {} tokens", launch_pool.creator_claimed_tokens);
    msg!("Remaining allocation: {} tokens", remaining_claimable);

    Ok(())
}
