use anchor_lang::prelude::*;

#[account]
pub struct UserPoint {
    /// User address
    pub user: Pubkey,

    /// Points consumed
    pub points_consumed: u64,

    /// Reserved space
    pub reserved: [u64; 8],
}

impl UserPoint {
    pub const SIZE: usize = 8 + // discriminator
        32 + // user
        8 + // points_consumed
        8 * 8; // reserved
}
