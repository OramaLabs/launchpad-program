use anchor_lang::prelude::*;

use crate::errors::LaunchpadError;

/// Calculate token allocations
pub fn calculate_token_allocations(total_supply: u64) -> Result<(u64, u64, u64)> {
    let creator_allocation = total_supply
        .checked_mul(crate::constants::CREATOR_ALLOCATION_PERCENT as u64)
        .ok_or(LaunchpadError::MathOverflow)?
        .checked_div(100)
        .ok_or(LaunchpadError::DivisionByZero)?;

    let sale_allocation = total_supply
        .checked_mul(crate::constants::SALE_ALLOCATION_PERCENT as u64)
        .ok_or(LaunchpadError::MathOverflow)?
        .checked_div(100)
        .ok_or(LaunchpadError::DivisionByZero)?;

    let liquidity_allocation = total_supply
        .checked_mul(crate::constants::LIQUIDITY_ALLOCATION_PERCENT as u64)
        .ok_or(LaunchpadError::MathOverflow)?
        .checked_div(100)
        .ok_or(LaunchpadError::DivisionByZero)?;

    // Verify total
    let total = creator_allocation
        .checked_add(sale_allocation)
        .ok_or(LaunchpadError::MathOverflow)?
        .checked_add(liquidity_allocation)
        .ok_or(LaunchpadError::MathOverflow)?;

    require!(
        total == total_supply,
        LaunchpadError::InvalidTokenAllocation
    );

    Ok((creator_allocation, sale_allocation, liquidity_allocation))
}
