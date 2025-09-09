use anchor_lang::prelude::*;
use ruint::aliases::{U256, U512};

use crate::errors::LaunchpadError;


// L = Δx * sqrt(P) * sqrt(P_upper) / (sqrt(P_upper) - sqrt(P))
fn get_initial_liquidity_from_delta_base(
    base_amount: u64,
    sqrt_max_price: u128,
    sqrt_price: u128,
) -> Result<U512> {
    let delta = sqrt_max_price
        .checked_sub(sqrt_price)
        .ok_or(LaunchpadError::MathOverflow)?;
    let price_delta = U512::from(delta);

    let base = U512::from(base_amount);
    let sqrt_price = U512::from(sqrt_price);
    let sqrt_max_price = U512::from(sqrt_max_price);

    let prod = base
        .checked_mul(sqrt_price)
        .ok_or(LaunchpadError::MathOverflow)?
        .checked_mul(sqrt_max_price)
        .ok_or(LaunchpadError::MathOverflow)?;

    let liquidity = prod
        .checked_div(price_delta)
        .ok_or(LaunchpadError::MathOverflow)?;

    Ok(liquidity)
}

// L = Δy * 2^128 / (sqrt(P) - sqrt(P_lower))
fn get_initial_liquidity_from_delta_quote(
    quote_amount: u64,
    sqrt_min_price: u128,
    sqrt_price: u128,
) -> Result<u128> {
    let delta = sqrt_price
        .checked_sub(sqrt_min_price)
        .ok_or(LaunchpadError::MathOverflow)?;
    let price_delta = U256::from(delta);

    let quote = U256::from(quote_amount);
    let quote_shifted = quote
        .checked_shl(128)
        .ok_or(LaunchpadError::MathOverflow)?;

    let liquidity = quote_shifted
        .checked_div(price_delta)
        .ok_or(LaunchpadError::MathOverflow)?;

    return Ok(liquidity.to::<u128>())
}

pub fn get_liquidity_for_adding_liquidity(
    base_amount: u64,
    quote_amount: u64,
    sqrt_price: u128,
    min_sqrt_price: u128,
    max_sqrt_price: u128,
) -> Result<u128> {
    let liquidity_from_base =
        get_initial_liquidity_from_delta_base(base_amount, max_sqrt_price, sqrt_price)?;
    let liquidity_from_quote =
        get_initial_liquidity_from_delta_quote(quote_amount, min_sqrt_price, sqrt_price)?;
    if liquidity_from_base > U512::from(liquidity_from_quote) {
        Ok(liquidity_from_quote)
    } else {
        Ok(liquidity_from_base
            .try_into()
            .map_err(|_| LaunchpadError::TypeCastFailed)?)
    }
}
