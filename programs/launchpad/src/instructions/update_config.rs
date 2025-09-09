use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::LaunchpadError;
use crate::state::GlobalConfig;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateConfigParams {
    pub points_signer: Option<Pubkey>,
    pub points_per_sol: Option<u64>,
    pub min_target_sol: Option<u64>,
    pub max_target_sol: Option<u64>,
    pub min_duration: Option<i64>,
    pub max_duration: Option<i64>,
    pub paused: Option<bool>,
    pub min_stake_duration: Option<i64>,
    pub lb_pair: Option<Pubkey>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = global_config.admin == admin.key() @ LaunchpadError::Unauthorized,
    )]
    pub global_config: Box<Account<'info, GlobalConfig>>,
}

pub fn update_config(
    ctx: Context<UpdateConfig>,
    params: UpdateConfigParams,
) -> Result<()> {
    let config = &mut ctx.accounts.global_config;

    // Update configuration parameters
    if let Some(points_signer) = params.points_signer {
        config.points_signer = points_signer;
    }

    if let Some(points_per_sol) = params.points_per_sol {
        config.points_per_sol = points_per_sol;
    }

    if let Some(min_target_sol) = params.min_target_sol {
        config.min_target_sol = min_target_sol;
    }

    if let Some(max_target_sol) = params.max_target_sol {
        config.max_target_sol = max_target_sol;
    }

    if let Some(min_duration) = params.min_duration {
        config.min_duration = min_duration;
    }

    if let Some(max_duration) = params.max_duration {
        config.max_duration = max_duration;
    }

    if let Some(paused) = params.paused {
        config.paused = paused;
    }

    if let Some(min_stake_duration) = params.min_stake_duration {
        config.min_stake_duration = min_stake_duration;
    }

    if let Some(lb_pair) = params.lb_pair {
        config.lb_pair = lb_pair;
    }

    msg!("Global config updated successfully");

    Ok(())
}
