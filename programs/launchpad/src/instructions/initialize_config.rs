use anchor_lang::prelude::*;

use crate::constants::*;
use crate::state::GlobalConfig;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeConfigParams {
    pub points_signer: Pubkey,
    pub lb_pair: Pubkey,
    pub points_per_sol: Option<u64>,
    pub min_target_sol: Option<u64>,
    pub max_target_sol: Option<u64>,
    pub min_duration: Option<i64>,
    pub max_duration: Option<i64>,
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = GlobalConfig::SIZE,
        seeds = [GLOBAL_CONFIG_SEED],
        bump,
    )]
    pub global_config: Box<Account<'info, GlobalConfig>>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_config(
    ctx: Context<InitializeConfig>,
    params: InitializeConfigParams,
) -> Result<()> {
    let config = &mut ctx.accounts.global_config;

    // First set default values
    config.initialize_defaults(
        ctx.accounts.admin.key(),
        params.points_signer,
        params.lb_pair,
        ctx.bumps.global_config,
    );

    // Then override default values with parameters
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

    msg!("Global config initialized successfully");
    msg!("Admin: {}", config.admin);
    msg!("Points signer: {}", config.points_signer);
    msg!("Points per SOL: {}", config.points_per_sol);

    Ok(())
}
