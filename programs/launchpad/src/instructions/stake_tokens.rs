use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{TOKEN_VAULT, VAULT_AUTHORITY};
use crate::errors::LaunchpadError;
use crate::events::{TokensStaked};
use crate::state::{GlobalConfig, StakingPosition};

#[derive(Accounts)]
#[instruction(amount: u64, lock_duration: i64)]
pub struct StakeTokens<'info> {
    /// User who wants to stake tokens
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

    /// Token mint of the token to be staked
    pub token_mint: Account<'info, Mint>,

    /// User's token account (source of tokens)
    #[account(
        mut,
        token::mint = token_mint,
        token::authority = user,
    )]
    pub user_token_account: Box<Account<'info, TokenAccount>>,

    /// Program's token vault to hold staked tokens
    #[account(
        init_if_needed,
        payer = user,
        seeds = [TOKEN_VAULT, vault_authority.key().as_ref(), token_mint.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = vault_authority,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,

    /// Staking position account for this user and token
    #[account(
        init_if_needed,
        payer = user,
        space = StakingPosition::SIZE,
        seeds = [
            StakingPosition::SEED,
            user.key().as_ref(),
            token_mint.key().as_ref()
        ],
        bump,
    )]
    pub staking_position: Box<Account<'info, StakingPosition>>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// System program
    pub system_program: Program<'info, System>,

    /// Rent sysvar
    pub rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct StakeTokensParams {
    pub amount: u64,
    pub lock_duration: i64,
}

pub fn stake_tokens(
    ctx: Context<StakeTokens>,
    params: StakeTokensParams,
) -> Result<()> {
    let StakeTokensParams { amount, lock_duration } = params;

    // Validate inputs
    require!(amount > 0, LaunchpadError::CannotStakeZeroTokens);

    let global_config = &ctx.accounts.global_config;
    let current_time = Clock::get()?.unix_timestamp;

    // Validate staking parameters
    global_config.validate_stake_params(lock_duration)?;

    // Transfer tokens from user to vault
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.token_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(transfer_ctx, amount)?;

    // Check if this is a new staking position or adding to existing one
    let is_new_position = ctx.accounts.staking_position.staked_amount == 0
        && ctx.accounts.staking_position.user == Pubkey::default();

    let bump = ctx.bumps.staking_position;

    if is_new_position {
        // Initialize new staking position
        ctx.accounts.staking_position.initialize(
            ctx.accounts.user.key(),
            ctx.accounts.token_mint.key(),
            amount,
            lock_duration,
            current_time,
            bump,
        )?;
    } else {
        // Update existing staking position (add to existing stake)
        ctx.accounts.staking_position.update_stake(
            amount,
            lock_duration,
            current_time,
        )?;
    }

    // Emit stake event (without reward fields as rewards are handled off-chain)
    emit!(TokensStaked {
        user: ctx.accounts.user.key(),
        position: ctx.accounts.staking_position.key(),
        token_mint: ctx.accounts.token_mint.key(),
        amount,
        total_staked: ctx.accounts.staking_position.staked_amount,
        lock_duration,
        unlock_time: ctx.accounts.staking_position.unlock_time,
        stake_time: current_time,
        is_additional_stake: !is_new_position,
    });

    msg!(
        "User {} staked {} tokens of mint {} for {} seconds",
        ctx.accounts.user.key(),
        amount,
        ctx.accounts.token_mint.key(),
        lock_duration
    );

    Ok(())
}
