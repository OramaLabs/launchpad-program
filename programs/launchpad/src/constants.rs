// ===== Seeds =====
pub const GLOBAL_CONFIG_SEED: &[u8] = b"global_config";
pub const LAUNCH_POOL_SEED: &[u8] = b"launch_pool";
pub const USER_POINT_SEED: &[u8] = b"user_point";
pub const USER_POSITION_SEED: &[u8] = b"user_position";
pub const USER_DIVIDEND_SEED: &[u8] = b"user_dividend";
pub const VAULT_AUTHORITY: &[u8] = b"vault_authority";
pub const TOKEN_VAULT: &[u8] = b"token_vault";
pub const TOKEN_MINT_SEED: &[u8] = b"token_mint";

// ===== Token Configuration =====
/// Token decimals (standard SPL token)
pub const TOKEN_DECIMALS: u8 = 6;

/// Total supply: 1 billion tokens
pub const TOTAL_SUPPLY: u64 = 1_000_000_000 * 10u64.pow(TOKEN_DECIMALS as u32);

// ===== Token Allocation =====
/// Creator allocation: 30%
pub const CREATOR_ALLOCATION_PERCENT: u8 = 30;

/// Sale allocation: 50%
pub const SALE_ALLOCATION_PERCENT: u8 = 50;

/// Liquidity allocation: 20%
pub const LIQUIDITY_ALLOCATION_PERCENT: u8 = 20;

// ===== Launch Parameters =====
/// Default target: 100 SOL
pub const DEFAULT_TARGET_SOL: u64 = 100 * anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;

/// Default launch duration: 12 hours
pub const DEFAULT_LAUNCH_DURATION: i64 = 12 * 60 * 60;

// ===== Creator Lock Configuration =====
/// Default creator lock duration: 30 days (in seconds)
pub const DEFAULT_CREATOR_LOCK_DURATION: i64 = 30 * 24 * 60 * 60;

/// Default creator linear unlock duration: 90 days (in seconds)
pub const DEFAULT_CREATOR_LINEAR_UNLOCK_DURATION: i64 = 90 * 24 * 60 * 60;

/// Maximum contribution per user (prevent monopolization)
pub const MAX_CONTRIBUTION_PER_USER: u64 = 3 * anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;

/// Minimum contribution per user
pub const MIN_CONTRIBUTION_PER_USER: u64 = anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL / 10; // 0.1 SOL

/// sqrt(100000000000/200000000000000) * 2^64
pub const SQRT_PRICE: u128 = 412481737123559485;
