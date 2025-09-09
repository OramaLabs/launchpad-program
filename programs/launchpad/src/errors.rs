use anchor_lang::prelude::*;

#[error_code]
pub enum LaunchpadError {
    // ===== Permission Errors =====
    #[msg("Unauthorized: Only admin can perform this action")]
    Unauthorized,

    #[msg("Not the creator of this launch pool")]
    NotCreator,

    // ===== Status Errors =====
    #[msg("Invalid status for this operation")]
    InvalidStatus,

    #[msg("Launch pool is not active")]
    LaunchNotActive,

    #[msg("Launch pool not migrated")]
    NotMigrated,

    #[msg("Launch pool has failed")]
    LaunchFailed,

    #[msg("Platform is currently paused")]
    PlatformPaused,

    // ===== Time Errors =====
    #[msg("Launch has not started yet")]
    NotStarted,

    #[msg("Launch time window has expired")]
    TimeWindowExpired,

    #[msg("Too early to finalize")]
    TooEarlyToFinalize,

    // ===== Parameter Errors =====
    #[msg("Invalid target amount")]
    InvalidTargetAmount,

    #[msg("Invalid duration")]
    InvalidDuration,

    #[msg("Invalid token allocation")]
    InvalidTokenAllocation,

    #[msg("Invalid points amount")]
    InvalidPointsAmount,

    #[msg("Insufficient points balance")]
    InsufficientPoints,

    #[msg("Invalid contribution amount")]
    InvalidContribution,

    // ===== Signature Errors =====
    #[msg("Invalid signature")]
    InvalidSignature,

    #[msg("Invalid instruction index")]
    InvalidInstructionIndex,

    // ===== Math Errors =====
    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Division by zero")]
    DivisionByZero,

    // ===== Claim Errors =====
    #[msg("Nothing to claim")]
    NothingToClaim,

    #[msg("Already claimed")]
    AlreadyClaimed,

    #[msg("Invalid token mint")]
    InvalidTokenMint,

    #[msg("Invalid launch status")]
    InvalidLaunchStatus,

    #[msg("Invalid quote mint")]
    InvalidQuoteMint,

    #[msg("Invalid token vault")]
    InvalidTokenVault,

    #[msg("Invalid quote vault")]
    InvalidQuoteVault,

    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,

    // ===== Staking Errors =====
    #[msg("Invalid stake duration")]
    InvalidStakeDuration,

    #[msg("Stake not unlocked yet")]
    StakeNotUnlocked,

    #[msg("No stake position found")]
    NoStakeFound,

    #[msg("Invalid token mint for staking")]
    InvalidStakingTokenMint,

    #[msg("Cannot stake zero tokens")]
    CannotStakeZeroTokens,

    #[msg("Type conversion failed")]
    TypeCastFailed,

    #[msg("Invalid lb_pair address")]
    InvalidLbPair,
}
