use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{TOKEN_VAULT, VAULT_AUTHORITY};
use crate::errors::LaunchpadError;
use crate::events::{TokensUnstaked};
use crate::state::{GlobalConfig, StakingPosition};

#[derive(Accounts)]
pub struct UnstakeTokens<'info> {
    /// User who wants to unstake tokens
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

    /// Global configuration account
    #[account(
        seeds = [GlobalConfig::SEED],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// Token mint of the staked token
    pub token_mint: Account<'info, Mint>,

    /// User's token account (destination for tokens)
    #[account(
        mut,
        token::mint = token_mint,
        token::authority = user,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Program's token vault holding staked tokens
    #[account(
        mut,
        token::mint = token_mint,
        token::authority = staking_position,
        seeds = [TOKEN_VAULT, vault_authority.key().as_ref(), token_mint.key().as_ref()],
        bump,
    )]
    pub token_vault: Account<'info, TokenAccount>,

    /// Staking position account for this user and token
    #[account(
        mut,
        close = user,
        seeds = [
            StakingPosition::SEED,
            user.key().as_ref(),
            token_mint.key().as_ref()
        ],
        bump = staking_position.bump,
        constraint = staking_position.user == user.key() @ LaunchpadError::NoStakeFound,
        constraint = staking_position.token_mint == token_mint.key() @ LaunchpadError::InvalidStakingTokenMint,
    )]
    pub staking_position: Account<'info, StakingPosition>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>,
}

pub fn unstake_tokens(ctx: Context<UnstakeTokens>) -> Result<()> {
    let staking_position = &ctx.accounts.staking_position;
    let current_time = Clock::get()?.unix_timestamp;

    // Check if tokens can be unstaked (lock period has passed)
    require!(
        staking_position.can_unstake(current_time),
        LaunchpadError::StakeNotUnlocked
    );

    // Calculate total amount to transfer (staked amount + unclaimed rewards)
    let total_to_transfer = staking_position.staked_amount;

    // Prepare seeds for PDA signing
    let user_key = ctx.accounts.user.key();
    let token_mint_key = ctx.accounts.token_mint.key();
    let bump = staking_position.bump;
    let seeds = &[
        StakingPosition::SEED,
        user_key.as_ref(),
        token_mint_key.as_ref(),
        &[bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // Transfer tokens back to user
    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.token_vault.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.staking_position.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(transfer_ctx, total_to_transfer)?;

    // Calculate duration staked and rewards earned
    let duration_staked = current_time - staking_position.stake_time;
    let rewards_earned = total_to_transfer.saturating_sub(staking_position.staked_amount);

    // Emit improved unstake event
    emit!(TokensUnstaked {
        user: ctx.accounts.user.key(),
        position: staking_position.key(),
        token_mint: ctx.accounts.token_mint.key(),
        staked_amount: staking_position.staked_amount,
        rewards_earned,
        total_received: total_to_transfer,
        duration_staked,
        unstake_time: current_time,
    });

    msg!(
        "User {} unstaked {} tokens from mint {}",
        ctx.accounts.user.key(),
        staking_position.staked_amount,
        ctx.accounts.token_mint.key()
    );

    Ok(())
}
