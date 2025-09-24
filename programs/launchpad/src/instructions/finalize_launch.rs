use anchor_lang::prelude::*;
use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;

use crate::constants::LAUNCH_POOL_SEED;
use crate::errors::LaunchpadError;
use crate::state::{LaunchPool, LaunchStatus};
use crate::utils::validation::check_can_finalize;
use crate::events::{LaunchFinalized, LaunchStatusChanged};

#[derive(Accounts)]
pub struct FinalizeLaunch<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [LAUNCH_POOL_SEED, launch_pool.creator.as_ref(), &launch_pool.index.to_le_bytes()],
        bump = launch_pool.bump,
        constraint = launch_pool.is_active() @ LaunchpadError::LaunchNotActive,
    )]
    pub launch_pool: Box<Account<'info, LaunchPool>>,
}

pub fn finalize_launch(ctx: Context<FinalizeLaunch>) -> Result<()> {
    let launch_pool = &mut ctx.accounts.launch_pool;
    let clock = Clock::get()?;

    // Validate if can finalize
    check_can_finalize(launch_pool, clock.unix_timestamp)?;

    let previous_status = launch_pool.status as u8;

    // Check if target is reached
    let success = launch_pool.raised_sol >= launch_pool.target_sol;

    if success {
        // Success - mark as successful status, waiting for subsequent create_meteora_pool call
        launch_pool.status = LaunchStatus::Success;

        msg!("Launch finalized successfully!");
        msg!("Total raised: {} SOL", launch_pool.raised_sol / LAMPORTS_PER_SOL);
        msg!("Next step: Call create_meteora_pool to create liquidity pool");
    } else {
        // Failed
        launch_pool.status = LaunchStatus::Failed;

        msg!("Launch failed to reach target");
        msg!("Raised: {} / {} SOL",
            launch_pool.raised_sol / LAMPORTS_PER_SOL,
            launch_pool.target_sol / LAMPORTS_PER_SOL
        );
    }

    launch_pool.finalized_time = clock.unix_timestamp;

    // Emit status change event
    emit!(LaunchStatusChanged {
        pool: launch_pool.key(),
        previous_status,
        new_status: launch_pool.status as u8,
        raised_amount: launch_pool.raised_sol,
        target_amount: launch_pool.target_sol,
        timestamp: clock.unix_timestamp,
    });

    // Emit launch finalized event
    emit!(LaunchFinalized {
        pool: launch_pool.key(),
        creator: launch_pool.creator,
        success,
        raised_amount: launch_pool.raised_sol,
        target_amount: launch_pool.target_sol,
        liquidity_amount: launch_pool.liquidity_sol,
        excess_amount: launch_pool.excess_sol,
        participants_count: launch_pool.participants_count,
        total_points_consumed: launch_pool.total_points_consumed,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
