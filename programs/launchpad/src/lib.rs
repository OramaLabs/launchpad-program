#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

mod const_pda;
pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_program!(dlmm);

declare_id!("7E22dUYERWbyaqGDTKeU7NfYPRnBuAaVULXJgafxsBHq");

#[program]
pub mod launchpad {
    use super::*;

    /// Initialize global configuration
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        params: InitializeConfigParams,
    ) -> Result<()> {
        instructions::initialize_config(ctx, params)
    }

    /// Initialize a new token launch pool
    pub fn initialize_launch(
        ctx: Context<InitializeLaunch>,
        params: InitializeLaunchParams,
    ) -> Result<()> {
        instructions::initialize_launch(ctx, params)
    }

    /// Participate in the launch using points
    pub fn participate_with_points(
        ctx: Context<ParticipateWithPoints>,
        points_to_use: u64,
        total_points: u64,
        signature: [u8; 64],
    ) -> Result<()> {
        instructions::participate_with_points(
            ctx,
            points_to_use,
            total_points,
            signature,
        )
    }

    /// Finalize the launch (success or failure)
    pub fn finalize_launch(ctx: Context<FinalizeLaunch>) -> Result<()> {
        instructions::finalize_launch(ctx)
    }

    /// Update global configuration (admin only)
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        params: UpdateConfigParams,
    ) -> Result<()> {
        instructions::update_config(ctx, params)
    }

    /// Create Meteora liquidity pool after successful launch
    pub fn create_meteora_pool(ctx: Context<DammV2>, sqrt_price: u128) -> Result<()> {
        ctx.accounts.create_pool(sqrt_price)
    }

    /// Claim user rewards (tokens and excess SOL)
    pub fn claim_user_rewards(ctx: Context<ClaimUserRewards>) -> Result<()> {
        instructions::claim_user_rewards(ctx)
    }

    /// Claim creator tokens (with vesting)
    pub fn claim_creator_tokens(ctx: Context<ClaimCreatorTokens>) -> Result<()> {
        instructions::claim_creator_tokens(ctx)
    }

    /// Stake tokens with lock duration
    pub fn stake_tokens(
        ctx: Context<StakeTokens>,
        params: StakeTokensParams,
    ) -> Result<()> {
        instructions::stake_tokens(ctx, params)
    }

    /// Unstake tokens and claim all rewards
    pub fn unstake_tokens(ctx: Context<UnstakeTokens>) -> Result<()> {
        instructions::unstake_tokens(ctx)
    }

    pub fn claim_pool_fee(
        ctx: Context<ClaimPositionFee>,
    ) -> Result<()> {
        ctx.accounts
            .claim_position_fee()?;

        Ok(())
    }

    /// Swap tokens with optional fee
    pub fn swap<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, DlmmSwap<'info>>,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        instructions::handle_dlmm_swap(ctx, amount_in, min_amount_out)
    }
}
