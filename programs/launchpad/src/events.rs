use anchor_lang::prelude::*;

// =============================================================================
// LAUNCH POOL LIFECYCLE EVENTS
// =============================================================================

/// Event emitted when a new launch pool is initialized
#[event]
pub struct LaunchPoolInitialized {
    /// Launch pool address
    pub pool: Pubkey,
    /// Creator of the launch pool
    pub creator: Pubkey,
    /// Token mint address
    pub token_mint: Pubkey,
    /// Token name
    pub token_name: String,
    /// Token symbol
    pub token_symbol: String,
    /// Total token supply
    pub total_supply: u64,
    /// Target SOL to raise
    pub target_sol: u64,
    /// Launch duration in seconds
    pub duration: i64,
    /// Points per SOL ratio
    pub points_per_sol: u64,
    /// Creator lock duration
    pub creator_lock_duration: i64,
    /// Start timestamp
    pub start_time: i64,
    /// End timestamp
    pub end_time: i64,
}

/// Event emitted when a user participates in a launch pool
#[event]
pub struct ParticipationEvent {
    /// Launch pool address
    pub pool: Pubkey,
    /// User who participated
    pub user: Pubkey,
    /// Amount of SOL contributed
    pub sol_amount: u64,
    /// Amount of points used
    pub points_used: u64,
    /// User's total contribution so far
    pub total_contribution: u64,
    /// Pool's total raised amount after this contribution
    pub pool_raised_total: u64,
    /// Whether this is the user's first participation
    pub is_first_participation: bool,
    /// Current participant count
    pub participants_count: u32,
    /// Participation timestamp
    pub timestamp: i64,
}

/// Event emitted when launch status changes
#[event]
pub struct LaunchStatusChanged {
    /// Launch pool address
    pub pool: Pubkey,
    /// Previous status
    pub previous_status: u8, // LaunchStatus as u8
    /// New status
    pub new_status: u8, // LaunchStatus as u8
    /// Total amount raised at status change
    pub raised_amount: u64,
    /// Target amount
    pub target_amount: u64,
    /// Timestamp of status change
    pub timestamp: i64,
}

/// Event emitted when a launch pool is finalized
#[event]
pub struct LaunchFinalized {
    /// Launch pool address
    pub pool: Pubkey,
    /// Creator of the pool
    pub creator: Pubkey,
    /// Whether the launch was successful (reached target)
    pub success: bool,
    /// Total amount raised
    pub raised_amount: u64,
    /// Target amount
    pub target_amount: u64,
    /// Amount for liquidity
    pub liquidity_amount: u64,
    /// Excess amount (if over-funded)
    pub excess_amount: u64,
    /// Total participants
    pub participants_count: u32,
    /// Total points consumed
    pub total_points_consumed: u64,
    /// Finalization timestamp
    pub timestamp: i64,
}

// =============================================================================
// TOKEN CLAIM EVENTS
// =============================================================================

/// Event emitted when creator claims their tokens
#[event]
pub struct CreatorTokensClaimed {
    /// Launch pool address
    pub pool: Pubkey,
    /// Creator address
    pub creator: Pubkey,
    /// Token mint
    pub token_mint: Pubkey,
    /// Amount of tokens claimed in this transaction
    pub claimed_amount: u64,
    /// Total amount claimed so far
    pub total_claimed: u64,
    /// Total creator allocation
    pub total_allocation: u64,
    /// Remaining claimable amount
    pub remaining_claimable: u64,
    /// Whether fully unlocked
    pub fully_unlocked: bool,
    /// Claim timestamp
    pub timestamp: i64,
}

/// Event emitted when users claim their rewards (tokens + excess SOL)
#[event]
pub struct UserRewardsClaimed {
    /// Launch pool address
    pub pool: Pubkey,
    /// User address
    pub user: Pubkey,
    /// Token mint
    pub token_mint: Pubkey,
    /// Amount of tokens claimed
    pub tokens_claimed: u64,
    /// Amount of excess SOL claimed
    pub excess_sol_claimed: u64,
    /// User's total contribution
    pub user_contribution: u64,
    /// Pool's total raised amount
    pub pool_total_raised: u64,
    /// Claim timestamp
    pub timestamp: i64,
}

/// Event emitted when users get refunds for failed launch pools
#[event]
pub struct UserRefunded {
    /// Launch pool address
    pub pool: Pubkey,
    /// User address
    pub user: Pubkey,
    /// Token mint
    pub token_mint: Pubkey,
    /// Amount of SOL refunded
    pub refund_amount: u64,
    /// User's original contribution
    pub user_contribution: u64,
    /// Pool's total raised amount
    pub pool_total_raised: u64,
    /// Refund timestamp
    pub timestamp: i64,
}

// =============================================================================
// STAKING EVENTS (IMPROVED)
// =============================================================================

/// Event emitted when tokens are staked
#[event]
pub struct TokensStaked {
    /// User who staked the tokens
    pub user: Pubkey,
    /// Staking position account
    pub position: Pubkey,
    /// Token mint address of the staked token
    pub token_mint: Pubkey,
    /// Amount of tokens staked in this transaction
    pub amount: u64,
    /// Total amount staked in this position after this transaction
    pub total_staked: u64,
    /// Lock duration in seconds
    pub lock_duration: i64,
    /// Timestamp when tokens can be unlocked
    pub unlock_time: i64,
    /// Timestamp when stake was created
    pub stake_time: i64,
    /// Whether this is additional stake to existing position
    pub is_additional_stake: bool,
}

/// Event emitted when tokens are unstaked
#[event]
pub struct TokensUnstaked {
    /// User who unstaked the tokens
    pub user: Pubkey,
    /// Staking position account
    pub position: Pubkey,
    /// Token mint address of the unstaked token
    pub token_mint: Pubkey,
    /// Amount of tokens unstaked in this transaction
    pub unstaked_amount: u64,
    /// Amount of tokens still staked after this unstake
    pub remaining_staked: u64,
    /// Duration staked in seconds
    pub duration_staked: i64,
    /// Timestamp when unstake occurred
    pub unstake_time: i64,
}

/// Event emitted when liquidity pool is created on Meteora
#[event]
pub struct LiquidityPoolCreated {
    /// Launch pool address
    pub launch_pool: Pubkey,
    /// Meteora pool address
    pub meteora_pool: Pubkey,
    /// Token mint
    pub token_mint: Pubkey,
    /// Quote mint (WSOL)
    pub quote_mint: Pubkey,
    /// Amount of tokens added to liquidity
    pub token_amount: u64,
    /// Amount of SOL added to liquidity
    pub sol_amount: u64,
    /// LP token mint (if applicable)
    pub lp_token_mint: Pubkey,
    /// Creation timestamp
    pub timestamp: i64,
}

// =============================================================================
// SWAP EVENTS
// =============================================================================

/// Event emitted when a swap fee is charged
#[event]
pub struct SwapFeeCharged {
    /// User performing the swap
    pub user: Pubkey,
    /// Input token mint
    pub input_token_mint: Pubkey,
    /// Output token mint
    pub output_token_mint: Pubkey,
    /// Total amount input by user
    pub amount_in: u64,
    /// Fee amount charged
    pub fee_amount: u64,
    /// Actual amount used for swap after fee
    pub actual_swap_amount: u64,
    /// Amount of output tokens received by user
    pub amount_out: u64,
    /// Fee percentage in basis points (5 = 0.05%)
    pub fee_percentage: u16,
    /// Timestamp of the swap
    pub timestamp: i64,
}

// =============================================================================
// DIVIDEND EVENTS
// =============================================================================

/// Event emitted when user claims token dividends
#[event]
pub struct DividendClaimed {
    /// User address who claimed dividends
    pub user: Pubkey,
    /// Token mint for which dividends were claimed
    pub token_mint: Pubkey,
    /// Amount of dividends claimed in this transaction
    pub claimed_amount: u64,
    /// Total amount of dividends this user has claimed for this token
    pub total_claimed: u64,
    /// Signed total dividend amount used for verification
    pub signed_total_dividend: u64,
    /// Claim timestamp
    pub timestamp: i64,
}
