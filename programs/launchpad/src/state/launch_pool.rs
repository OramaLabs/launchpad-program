use anchor_lang::prelude::*;

#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub enum LaunchStatus {
    Initialized,    // Initialization complete, waiting to start
    Active,         // Fundraising in progress
    Success,        // Fundraising successful (reached 100 SOL)
    Failed,         // Fundraising failed (didn't reach 100 SOL within 12 hours)
    Migrated,       // Migrated to Meteora
}

impl Default for LaunchStatus {
    fn default() -> Self {
        LaunchStatus::Initialized
    }
}

#[account]
pub struct LaunchPool {
    /// Project creator
    pub creator: Pubkey,

    pub token_mint: Pubkey,
    pub token_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub quote_mint: Pubkey,

    /// Current status
    pub status: LaunchStatus,

    /// bump seed
    pub bump: u8,

    // ===== Token Allocation =====
    /// Total supply
    pub total_supply: u64,

    /// Creator allocation (30%)
    pub creator_allocation: u64,

    /// Sale allocation (50%)
    pub sale_allocation: u64,

    /// Liquidity allocation (20%)
    pub liquidity_allocation: u64,

    // ===== Fundraising Information =====
    /// Target fundraising amount (100 SOL)
    pub target_sol: u64,

    /// SOL raised
    pub raised_sol: u64,

    /// Actual SOL for liquidity (max 100 SOL)
    pub liquidity_sol: u64,

    /// Excess SOL (amount over 100 SOL)
    pub excess_sol: u64,

    // ===== Time Management =====
    /// Start time
    pub start_time: i64,

    /// End time (start time + 12 hours)
    pub end_time: i64,

    /// Actual finalization time
    pub finalized_time: i64,

    // ===== Points Configuration =====
    /// Points required per SOL
    pub points_per_sol: u64,

    /// Total points consumed
    pub total_points_consumed: u64,

    // ===== Participant Statistics =====
    /// Participant count
    pub participants_count: u32,

    // ===== Creator Lock Configuration =====
    /// Creator token lock duration (in seconds)
    pub creator_lock_duration: i64,

    /// Creator token linear unlock duration (in seconds)
    pub creator_linear_unlock_duration: i64,

    /// Creator token unlock start time (usually after project completion)
    pub creator_unlock_start_time: i64,

    /// Creator claimed token amount
    pub creator_claimed_tokens: u64,

    pub index: u64,

    /// Meteora position account (set after migration)
    pub position: Option<Pubkey>,

    /// Meteora position NFT account (set after migration)
    pub position_nft_account: Option<Pubkey>,

    /// Reserved space
    pub reserved: [u64; 4],
}

impl LaunchPool {
    pub const SIZE: usize = 8 + // discriminator
        32 + // creator
        32 + // token_mint
        32 + // token_mint_vault
        32 + // quote_mint_vault
        32 + // quote_mint
        1 + // status (enum)
        1 + // bump
        8 + // total_supply
        8 + // creator_allocation
        8 + // sale_allocation
        8 + // liquidity_allocation
        8 + // target_sol
        8 + // raised_sol
        8 + // liquidity_sol
        8 + // excess_sol
        8 + // start_time
        8 + // end_time
        8 + // finalized_time
        8 + // points_per_sol
        8 + // total_points_consumed
        4 + // participants_count
        8 + // creator_lock_duration
        8 + // creator_linear_unlock_duration
        8 + // creator_unlock_start_time
        8 + // creator_claimed_tokens
        8 + // index
        33 + // position (Option<Pubkey>)
        33 + // position_nft_account (Option<Pubkey>)
        8 * 4; // reserved (reduced to 4)

    /// Check if fundraising is in active status
    pub fn is_active(&self) -> bool {
        self.status == LaunchStatus::Active
    }

    /// Check if fundraising is successful
    pub fn is_success(&self) -> bool {
        self.status == LaunchStatus::Success
    }

    /// is migrated
    pub fn is_migrated(&self) -> bool {
        self.status == LaunchStatus::Migrated
    }

    /// Update fundraising progress
    pub fn update_raised_amount(&mut self, sol_amount: u64) -> Result<()> {
        self.raised_sol = self.raised_sol
            .checked_add(sol_amount)
            .ok_or(error!(crate::errors::LaunchpadError::MathOverflow))?;

        // Calculate actual SOL for liquidity and excess SOL
        if self.raised_sol > self.target_sol {
            self.liquidity_sol = self.target_sol;
            self.excess_sol = self.raised_sol - self.target_sol;
        } else {
            self.liquidity_sol = self.raised_sol;
            self.excess_sol = 0;
        }

        Ok(())
    }

    /// Calculate creator's total unlocked token amount (cumulative)
    fn calculate_total_unlocked_tokens(&self, current_time: i64) -> u64 {
        // If unlock start time is not set yet, return 0
        if self.creator_unlock_start_time == 0 {
            return 0;
        }

        let lock_end_time = self.creator_unlock_start_time + self.creator_lock_duration;

        // If still in lock period, return 0
        if current_time < lock_end_time {
            return 0;
        }

        // If linear unlock time is 0, unlock all immediately after lock period
        if self.creator_linear_unlock_duration == 0 {
            return self.creator_allocation;
        }

        let unlock_end_time = lock_end_time + self.creator_linear_unlock_duration;

        // If unlock period has passed, all tokens are available
        if current_time >= unlock_end_time {
            return self.creator_allocation;
        }

        // During linear unlock period, use high precision calculation to avoid precision loss
        let elapsed_unlock_time = (current_time - lock_end_time) as u128;
        let total_unlock_duration = self.creator_linear_unlock_duration as u128;
        let total_allocation = self.creator_allocation as u128;

        // Multiply first then divide to maintain precision
        let unlocked_amount = (elapsed_unlock_time * total_allocation) / total_unlock_duration;

        // Ensure not exceeding total allocation
        unlocked_amount.min(total_allocation) as u64
    }

    /// Calculate creator's current new claimable token amount (excluding claimed portion)
    pub fn calculate_creator_claimable_amount(&self, current_time: i64) -> u64 {
        // Calculate total cumulative claimable amount
        let total_unlocked = self.calculate_total_unlocked_tokens(current_time);

        // Subtract claimed amount to get new claimable amount
        total_unlocked.saturating_sub(self.creator_claimed_tokens)
    }

    /// Check if in creator token lock period
    pub fn is_creator_tokens_locked(&self, current_time: i64) -> bool {
        if self.creator_unlock_start_time == 0 {
            return true; // Unlocking has not started yet
        }

        current_time < (self.creator_unlock_start_time + self.creator_lock_duration)
    }

    /// Get creator token unlock status information
    pub fn get_creator_unlock_info(&self, current_time: i64) -> (i64, i64, u64, bool) {
        let lock_end_time = self.creator_unlock_start_time + self.creator_lock_duration;
        let unlock_end_time = lock_end_time + self.creator_linear_unlock_duration;
        let claimable_amount = self.calculate_creator_claimable_amount(current_time);
        let is_locked = self.is_creator_tokens_locked(current_time);

        (lock_end_time, unlock_end_time, claimable_amount, is_locked)
    }
}
