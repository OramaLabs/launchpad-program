use anchor_lang::prelude::*;
use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;

use crate::errors::LaunchpadError;
use crate::state::{LaunchPool, LaunchStatus};

/// Validate if within time window
pub fn check_time_window(pool: &LaunchPool, current_time: i64) -> Result<()> {
    msg!("currrrrr {}", current_time);
    require!(
        current_time >= pool.start_time,
        LaunchpadError::NotStarted
    );

    require!(
        current_time <= pool.end_time,
        LaunchpadError::TimeWindowExpired
    );

    Ok(())
}

/// Validate if fundraising can be finalized
pub fn check_can_finalize(pool: &LaunchPool, current_time: i64) -> Result<()> {
    require!(
        pool.status == LaunchStatus::Active,
        LaunchpadError::LaunchNotActive
    );

    // Must wait until time window ends or target is reached
    let time_ended = current_time > pool.end_time;
    let target_reached = pool.raised_sol >= pool.target_sol;

    require!(
        time_ended || target_reached,
        LaunchpadError::TooEarlyToFinalize
    );

    Ok(())
}

/// Validate contribution amount
pub fn validate_contribution_amount(
    amount: u64,
    user_current: u64,
) -> Result<()> {
    require!(
        amount >= crate::constants::MIN_CONTRIBUTION_PER_USER,
        LaunchpadError::InvalidContribution
    );

    let total_contribution = user_current
        .checked_add(amount)
        .ok_or(LaunchpadError::MathOverflow)?;

    require!(
        total_contribution <= crate::constants::MAX_CONTRIBUTION_PER_USER,
        LaunchpadError::InvalidContribution
    );

    Ok(())
}

/// Validate points amount
pub fn validate_points_amount(
    points_to_use: u64,
    total_points: u64,
    points_consumed: u64,
) -> Result<()> {
    require!(
        points_to_use > 0,
        LaunchpadError::InvalidPointsAmount
    );

    require!(
        points_to_use <= total_points,
        LaunchpadError::InsufficientPoints
    );

    require!(
        points_to_use + points_consumed <= total_points,
        LaunchpadError::InsufficientPoints
    );

    Ok(())
}

pub fn calculate_sol_allowance(points: u64, points_per_sol: u64) -> Result<u64> {
    if points_per_sol == 0 {
        return err!(LaunchpadError::DivisionByZero);
    }

    let sol_amount = points
        .checked_mul(LAMPORTS_PER_SOL)
        .ok_or(LaunchpadError::MathOverflow)?
        .checked_div(points_per_sol)
        .ok_or(LaunchpadError::DivisionByZero)?;

    Ok(sol_amount)
}
