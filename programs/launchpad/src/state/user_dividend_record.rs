use anchor_lang::prelude::*;

#[account]
pub struct UserDividendRecord {
    /// User address
    pub user: Pubkey,

    /// Token mint address for the dividend
    pub token_mint: Pubkey,

    /// bump seed
    pub bump: u8,

    // ===== Dividend Information =====
    /// Total amount of dividends claimed by this user for this token
    pub total_claimed: u64,

    // ===== Time Records =====
    /// First claim time
    pub first_claimed_at: i64,

    /// Last claim time
    pub last_claimed_at: i64,

    /// Reserved space for future updates
    pub reserved: [u64; 8],
}

impl UserDividendRecord {
    pub const SIZE: usize = 8 + // discriminator
        32 + // user
        32 + // token_mint
        1 + // bump
        8 + // total_claimed
        8 + // first_claimed_at
        8 + // last_claimed_at
        8 * 8; // reserved

    /// Update claim information
    pub fn update_claim(
        &mut self,
        claimed_amount: u64,
        current_time: i64,
    ) -> Result<()> {
        // Update total claimed amount
        self.total_claimed = self.total_claimed
            .checked_add(claimed_amount)
            .ok_or(error!(crate::errors::LaunchpadError::MathOverflow))?;

        // Update timestamps
        self.last_claimed_at = current_time;
        
        if self.first_claimed_at == 0 {
            self.first_claimed_at = current_time;
        }

        Ok(())
    }

    /// Calculate claimable amount based on signed total and current claimed
    pub fn calculate_claimable(&self, signed_total_dividend: u64) -> Result<u64> {
        if signed_total_dividend < self.total_claimed {
            return Err(error!(crate::errors::LaunchpadError::InvalidAmount));
        }

        Ok(signed_total_dividend.saturating_sub(self.total_claimed))
    }
}