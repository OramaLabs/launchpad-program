use anchor_lang::prelude::*;

#[account]
pub struct GlobalConfig {
    /// Admin address (can update configuration)
    pub admin: Pubkey,

    /// Points signature verifier address
    pub points_signer: Pubkey,

    /// Points required per SOL (default configuration)
    pub points_per_sol: u64,

    /// Minimum fundraising target (SOL)
    pub min_target_sol: u64,

    /// Maximum fundraising target (SOL)
    pub max_target_sol: u64,

    /// Minimum fundraising duration (seconds)
    pub min_duration: i64,

    /// Maximum fundraising duration (seconds)
    pub max_duration: i64,

    /// Whether new launches are paused
    pub paused: bool,

    /// Minimum staking period (seconds) - 1 day
    pub min_stake_duration: i64,

    /// poll count
    pub pool_count: u64,

    /// Fixed lb_pair address for swap
    pub lb_pair: Pubkey,

    /// bump seed
    pub bump: u8,

    /// Reserved space
    pub reserved: [u64; 9],
}

impl GlobalConfig {
    pub const SIZE: usize = 8 + // discriminator
        32 + // admin
        32 + // points_signer
        8 + // points_per_sol
        8 + // min_target_sol
        8 + // max_target_sol
        8 + // min_duration
        8 + // max_duration
        1 + // paused
        8 + // min_stake_duration
        8 + // pool_count
        32 + // lb_pair
        1 + // bump
        8 * 9; // reserved

    pub const SEED: &'static [u8] = b"global_config";

    /// Initialize default configuration
    pub fn initialize_defaults(&mut self, admin: Pubkey, points_signer: Pubkey, lb_pair: Pubkey, bump: u8) {
        self.admin = admin;
        self.points_signer = points_signer;
        self.points_per_sol = 1000; // Default 1000 points = 1 SOL
        self.min_target_sol = 50_000_000_000; // 50 SOL
        self.max_target_sol = 500_000_000_000; // 500 SOL
        self.min_duration = 60 * 60; // 1 hour
        self.max_duration = 7 * 24 * 60 * 60; // 7 days
        self.paused = false;
        self.min_stake_duration = 24 * 60 * 60; // 1 day
        self.pool_count = 0;
        self.lb_pair = lb_pair;

        self.bump = bump;
    }

    /// Validate fundraising parameters
    pub fn validate_launch_params(&self, target_sol: u64, duration: i64) -> Result<()> {
        require!(
            !self.paused,
            crate::errors::LaunchpadError::PlatformPaused
        );

        require!(
            target_sol >= self.min_target_sol && target_sol <= self.max_target_sol,
            crate::errors::LaunchpadError::InvalidTargetAmount
        );

        require!(
            duration >= self.min_duration && duration <= self.max_duration,
            crate::errors::LaunchpadError::InvalidDuration
        );

        Ok(())
    }

    /// Validate staking parameters
    pub fn validate_stake_params(&self, duration: i64) -> Result<()> {
        require!(
            !self.paused,
            crate::errors::LaunchpadError::PlatformPaused
        );

        require!(
            duration >= self.min_stake_duration,
            crate::errors::LaunchpadError::InvalidStakeDuration
        );

        Ok(())
    }
}
