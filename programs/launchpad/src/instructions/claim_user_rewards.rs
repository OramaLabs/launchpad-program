use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer};

use crate::const_pda::const_authority::VAULT_BUMP;
use crate::constants::{USER_POSITION_SEED, VAULT_AUTHORITY};
use crate::state::{LaunchPool, LaunchStatus, UserPosition};
use crate::errors::LaunchpadError;
use crate::events::UserRewardsClaimed;

#[derive(Accounts)]
pub struct ClaimUserRewards<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// vault authority
    #[account(
        mut,
        seeds = [
            VAULT_AUTHORITY.as_ref(),
        ],
        bump,
    )]
    pub vault_authority: SystemAccount<'info>,

    #[account(
        mut,
        constraint = launch_pool.status == LaunchStatus::Failed || launch_pool.status == LaunchStatus::Migrated @ LaunchpadError::InvalidStatus,
    )]
    pub launch_pool: Box<Account<'info, LaunchPool>>,

    #[account(
        mut,
        seeds = [USER_POSITION_SEED, launch_pool.key().as_ref(), user.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.contributed_sol > 0 @ LaunchpadError::NothingToClaim
    )]
    pub user_position: Box<Account<'info, UserPosition>>,

    /// Pool's token vault
    #[account(
        mut,
        token::mint = launch_pool.token_mint.key(),
        token::authority = vault_authority,
        address = launch_pool.token_vault,
        constraint = launch_pool.token_vault == pool_token_vault.key() @ LaunchpadError::InvalidTokenVault
    )]
    pub pool_token_vault: Box<Account<'info, TokenAccount>>,

    /// Pool's quote vault (SOL)
    #[account(
        mut,
        token::mint = launch_pool.quote_mint.key(),
        token::authority = vault_authority,
        address = launch_pool.quote_vault,
        constraint = launch_pool.quote_vault == pool_quote_vault.key() @ LaunchpadError::InvalidQuoteVault
    )]
    pub pool_quote_vault: Box<Account<'info, TokenAccount>>,

    /// User's token account to receive tokens
    #[account(
        mut,
        token::mint = launch_pool.token_mint.key(),
        token::authority = user,
    )]
    pub user_token_account: Box<Account<'info, TokenAccount>>,

    /// User's quote account to receive excess SOL
    #[account(
        mut,
        token::mint = launch_pool.quote_mint.key(),
        token::authority = user,
    )]
    pub user_quote_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

/// Claim both tokens and excess SOL for users
pub fn claim_user_rewards(ctx: Context<ClaimUserRewards>) -> Result<()> {
    let pool = &mut ctx.accounts.launch_pool;
    let user_position: &mut Account<'_, UserPosition> = &mut ctx.accounts.user_position;
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    // Check if already claimed
    if user_position.tokens_claimed {
        return Err(LaunchpadError::AlreadyClaimed.into());
    }

    // Calculate tokens to claim
    let tokens_to_claim = calculate_user_token_allocation(
        user_position.contributed_sol,
        pool.raised_sol,
        pool.sale_allocation,
    )?;

    // Calculate excess SOL to claim
    let excess_sol_to_claim = if pool.excess_sol > 0 && !user_position.excess_sol_claimed {
        user_position.calculate_excess_sol(pool.excess_sol, pool.raised_sol)?
    } else {
        0
    };

    msg!("User claiming: {} tokens, {} excess SOL", tokens_to_claim, excess_sol_to_claim);

    let signer_seeds: &[&[&[u8]]] = &[&[VAULT_AUTHORITY, &[VAULT_BUMP]]];
    // Transfer tokens to user
    if tokens_to_claim > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_token_vault.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            tokens_to_claim,
        )?;
    }

    // Transfer excess SOL to user
    if excess_sol_to_claim > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_quote_vault.to_account_info(),
                    to: ctx.accounts.user_quote_account.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            excess_sol_to_claim,
        )?;

        token::close_account(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: ctx.accounts.user_quote_account.to_account_info(),
                destination: ctx.accounts.user.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ))?;
    }

    // Update user position
    user_position.tokens_claimed = true;
    if excess_sol_to_claim > 0 {
        user_position.excess_sol_claimed = true;
    }
    user_position.last_updated = current_time;

    // Emit user rewards claimed event
    emit!(UserRewardsClaimed {
        pool: pool.key(),
        user: ctx.accounts.user.key(),
        token_mint: pool.token_mint,
        tokens_claimed: tokens_to_claim,
        excess_sol_claimed: excess_sol_to_claim,
        user_contribution: user_position.contributed_sol,
        pool_total_raised: pool.raised_sol,
        timestamp: current_time,
    });

    msg!("User rewards claimed successfully");

    Ok(())
}

/// Calculate user's token allocation based on their SOL contribution
fn calculate_user_token_allocation(
    user_contributed_sol: u64,
    total_raised_sol: u64,
    sale_allocation: u64,
) -> Result<u64> {
    if total_raised_sol == 0 {
        return Ok(0);
    }

    // Calculate user's share of the sale allocation
    // user_tokens = (user_sol / total_sol) * sale_allocation
    let user_tokens = (user_contributed_sol as u128)
        .checked_mul(sale_allocation as u128)
        .ok_or(LaunchpadError::MathOverflow)?
        .checked_div(total_raised_sol as u128)
        .ok_or(LaunchpadError::MathOverflow)?;

    Ok(user_tokens as u64)
}
