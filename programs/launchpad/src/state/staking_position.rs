use anchor_lang::prelude::*;

#[account]
pub struct StakingPosition {
    /// User who owns this staking position
    pub user: Pubkey,

    /// Token mint address of the staked token
    pub token_mint: Pubkey,

    /// Amount of tokens staked
    pub staked_amount: u64,

    /// Lock duration in seconds
    pub lock_duration: i64,

    /// Timestamp when tokens were staked
    pub stake_time: i64,

    /// Timestamp when tokens can be unstaked
    pub unlock_time: i64,

    /// Bump seed for PDA
    pub bump: u8,

    /// Reserved space for future upgrades
    pub reserved: [u64; 8],
}

impl StakingPosition {
    pub const SIZE: usize = 8 + // discriminator
        32 + // user
        32 + // token_mint
        8 +  // staked_amount
        8 +  // lock_duration
        8 +  // stake_time
        8 +  // unlock_time
        1 +  // bump
        8 * 8; // reserved

    pub const SEED: &'static [u8] = b"staking_position";

    /// Check if the staking position can be unstaked
    pub fn can_unstake(&self, current_time: i64) -> bool {
        current_time >= self.unlock_time
    }

    /// Initialize staking position
    pub fn initialize(
        &mut self,
        user: Pubkey,
        token_mint: Pubkey,
        staked_amount: u64,
        lock_duration: i64,
        current_time: i64,
        bump: u8,
    ) {
        self.user = user;
        self.token_mint = token_mint;
        self.staked_amount = staked_amount;
        self.lock_duration = lock_duration;
        self.stake_time = current_time;
        self.unlock_time = current_time + lock_duration;
        self.bump = bump;
        self.reserved = [0; 8];
    }
}
