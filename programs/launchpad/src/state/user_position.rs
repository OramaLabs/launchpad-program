use anchor_lang::prelude::*;

#[account]
pub struct UserPosition {
    /// User address
    pub user: Pubkey,

    /// Associated launch pool
    pub pool: Pubkey,

    /// bump seed
    pub bump: u8,

    // ===== Contribution Information =====
    /// Amount of SOL contributed
    pub contributed_sol: u64,

    /// Points consumed
    pub points_consumed: u64,

    // ===== Claim Status =====
    /// Whether excess SOL has been claimed
    pub excess_sol_claimed: bool,

    /// Whether tokens have been claimed
    pub tokens_claimed: bool,

    /// Whether refunded (failure case)
    pub refunded: bool,

    // ===== Time Records =====
    /// Participation time
    pub participated_at: i64,

    /// Last updated time
    pub last_updated: i64,

    /// Reserved space
    pub reserved: [u64; 8],
}

impl UserPosition {
    pub const SIZE: usize = 8 + // discriminator
        32 + // user
        32 + // pool
        1 + // bump
        8 + // contributed_sol
        8 + // points_consumed
        1 + // excess_sol_claimed
        1 + // tokens_claimed
        1 + // refunded
        8 + // participated_at
        8 + // last_updated
        8 * 8; // reserved

    /// Check if can claim excess SOL
    pub fn can_claim_excess_sol(&self) -> bool {
        self.contributed_sol > 0 && !self.excess_sol_claimed && !self.refunded
    }

    /// Check if can refund
    pub fn can_refund(&self) -> bool {
        self.contributed_sol > 0 && !self.refunded && !self.tokens_claimed
    }

    /// Calculate deserved excess SOL
    pub fn calculate_excess_sol(&self, pool_excess: u64, pool_raised: u64) -> Result<u64> {
        // Allocate excess SOL proportionally
        // user_excess = (contributed_sol / raised_sol) * excess_sol
        if pool_raised == 0 {
            return Ok(0);
        }

        let user_share = (self.contributed_sol as u128)
            .checked_mul(pool_excess as u128)
            .ok_or(error!(crate::errors::LaunchpadError::MathOverflow))?
            .checked_div(pool_raised as u128)
            .ok_or(error!(crate::errors::LaunchpadError::MathOverflow))?;

        Ok(user_share as u64)
    }

    /// Update participation information
    pub fn update_participation(
        &mut self,
        sol_amount: u64,
        points: u64,
        current_time: i64,
    ) -> Result<()> {
        self.contributed_sol = self.contributed_sol
            .checked_add(sol_amount)
            .ok_or(error!(crate::errors::LaunchpadError::MathOverflow))?;

        self.points_consumed = self.points_consumed
            .checked_add(points)
            .ok_or(error!(crate::errors::LaunchpadError::MathOverflow))?;

        self.last_updated = current_time;

        if self.participated_at == 0 {
            self.participated_at = current_time;
        }

        Ok(())
    }
}
