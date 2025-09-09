use anchor_lang::prelude::*;
use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::metadata::{
    create_metadata_accounts_v3,
    CreateMetadataAccountsV3,
    Metadata,
};
use mpl_token_metadata::types::DataV2;

use crate::constants::*;
use crate::state::{GlobalConfig, LaunchPool, LaunchStatus};
use crate::utils::token::calculate_token_allocations;
use crate::events::LaunchPoolInitialized;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeLaunchParams {
    pub token_name: String,
    pub token_symbol: String,
    pub token_uri: String,
    pub target_sol: Option<u64>,  // Use default 100 SOL if not provided
    pub duration: Option<i64>,    // Use default 12 hours if not provided
    pub lock_duration: Option<i64>,  // Creator token lock duration (in seconds)
    pub linear_unlock_duration: Option<i64>,  // Creator token linear unlock duration (in seconds)
    pub start_time: Option<i64>, // start time
}

#[derive(Accounts)]
#[instruction(params: InitializeLaunchParams)]
pub struct InitializeLaunch<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    /// Global configuration account
    #[account(
        mut,
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
    )]
    pub global_config: Box<Account<'info, GlobalConfig>>,

    /// Launch pool account
    #[account(
        init,
        payer = creator,
        space = LaunchPool::SIZE,
        seeds = [LAUNCH_POOL_SEED, creator.key().as_ref(), &global_config.pool_count.to_le_bytes()],
        bump,
    )]
    pub launch_pool: Box<Account<'info, LaunchPool>>,

    /// vault authority
    #[account(
        mut,
        seeds = [
            VAULT_AUTHORITY.as_ref(),
        ],
        bump,
    )]
    pub vault_authority: SystemAccount<'info>,

    #[account(
        init,
        payer = creator,
        seeds = [TOKEN_MINT_SEED, launch_pool.key().as_ref()],
        bump,
        mint::decimals = TOKEN_DECIMALS,
        mint::authority = launch_pool.key(),
        mint::freeze_authority = launch_pool.key(),
    )]
    pub token_mint: Account<'info, Mint>,

    /// Launch pool token vault
    #[account(
        init_if_needed,
        payer = creator,
        seeds = [TOKEN_VAULT, vault_authority.key().as_ref(), token_mint.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = vault_authority,
    )]
    pub token_vault: Account<'info, TokenAccount>,

    /// CHECK: WSOL mint (verified by address)
    #[account(
        address = anchor_spl::token::spl_token::native_mint::ID
    )]
    pub wsol_mint: Account<'info, Mint>,

    /// Launch pool WSOL vault (for storing raised SOL)
    #[account(
        init_if_needed,
        payer = creator,
        seeds = [TOKEN_VAULT, vault_authority.key().as_ref(), wsol_mint.key().as_ref()],
        bump,
        token::mint = wsol_mint,
        token::authority = vault_authority,
    )]
    pub wsol_vault: Account<'info, TokenAccount>,

    /// Token metadata account
    /// CHECK: Validated by Metaplex program
    #[account(
        mut,
        seeds = [
            b"metadata",
            metadata_program.key().as_ref(),
            token_mint.key().as_ref(),
        ],
        seeds::program = metadata_program.key(),
        bump,
    )]
    pub metadata: UncheckedAccount<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// Associated Token program
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// Metadata program
    pub metadata_program: Program<'info, Metadata>,

    /// System program
    pub system_program: Program<'info, System>,

    /// Rent
    pub rent: Sysvar<'info, Rent>,
}

pub fn initialize_launch(
    ctx: Context<InitializeLaunch>,
    params: InitializeLaunchParams,
) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    let launch_pool = &mut ctx.accounts.launch_pool;
    let token_mint = &ctx.accounts.token_mint;
    let token_vault = &ctx.accounts.token_vault;
    let wsol_mint = &ctx.accounts.wsol_mint;
    let wsol_vault = &ctx.accounts.wsol_vault;
    let creator = &ctx.accounts.creator;
    let clock = Clock::get()?;

    // Validate parameters
    let target_sol = params.target_sol.unwrap_or(DEFAULT_TARGET_SOL);
    let duration = params.duration.unwrap_or(DEFAULT_LAUNCH_DURATION);
    let lock_duration = params.lock_duration.unwrap_or(DEFAULT_CREATOR_LOCK_DURATION);
    let linear_unlock_duration = params.linear_unlock_duration.unwrap_or(DEFAULT_CREATOR_LINEAR_UNLOCK_DURATION);
    let start_time = params.start_time.unwrap_or(clock.unix_timestamp);

    global_config.validate_launch_params(target_sol, duration)?;

    // Initialize launch pool
    launch_pool.creator = creator.key();
    launch_pool.token_mint = token_mint.key();
    launch_pool.token_vault = token_vault.key();
    launch_pool.quote_mint = wsol_mint.key();
    launch_pool.quote_vault = wsol_vault.key();
    launch_pool.status = LaunchStatus::Initialized;
    launch_pool.index = global_config.pool_count;
    launch_pool.bump = ctx.bumps.launch_pool;

    // Set token allocation
    launch_pool.total_supply = TOTAL_SUPPLY;
    let (creator_allocation, sale_allocation, liquidity_allocation) =
        calculate_token_allocations(TOTAL_SUPPLY)?;

    launch_pool.creator_allocation = creator_allocation;
    launch_pool.sale_allocation = sale_allocation;
    launch_pool.liquidity_allocation = liquidity_allocation;

    // Set fundraising parameters
    launch_pool.target_sol = target_sol;
    launch_pool.raised_sol = 0;
    launch_pool.liquidity_sol = 0;
    launch_pool.excess_sol = 0;

    // Set time parameters
    launch_pool.start_time = start_time;
    launch_pool.end_time = start_time + duration;
    launch_pool.finalized_time = 0;

    // Set points configuration
    launch_pool.points_per_sol = global_config.points_per_sol;
    launch_pool.total_points_consumed = 0;

    // Set creator lock configuration
    launch_pool.creator_lock_duration = lock_duration;
    launch_pool.creator_linear_unlock_duration = linear_unlock_duration;
    // Set unlock start time to 0, will be updated after project completion
    launch_pool.creator_unlock_start_time = 0;
    // Initialize claimed amount to 0
    launch_pool.creator_claimed_tokens = 0;

    // Initialize statistics
    launch_pool.participants_count = 0;

    // Mint all tokens to vault
    let creator_key = ctx.accounts.creator.key();
    let seeds = &[
        LAUNCH_POOL_SEED,
        creator_key.as_ref(),
        &global_config.pool_count.to_le_bytes(),
        &[launch_pool.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::MintTo {
                mint: ctx.accounts.token_mint.to_account_info(),
                to: ctx.accounts.token_vault.to_account_info(),
                authority: launch_pool.to_account_info(),
            },
            signer_seeds,
        ),
        TOTAL_SUPPLY,
    )?;

    // Create metadata
    let metadata_accounts = CreateMetadataAccountsV3 {
        metadata: ctx.accounts.metadata.to_account_info(),
        mint: ctx.accounts.token_mint.to_account_info(),
        mint_authority: launch_pool.to_account_info(),
        payer: ctx.accounts.creator.to_account_info(),
        update_authority: launch_pool.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        rent: ctx.accounts.rent.to_account_info(),
    };

    let data = DataV2 {
        name: params.token_name.clone(),
        symbol: params.token_symbol.clone(),
        uri: params.token_uri,
        seller_fee_basis_points: 0,
        creators: None,
        collection: None,
        uses: None,
    };

    create_metadata_accounts_v3(
        CpiContext::new_with_signer(
            ctx.accounts.metadata_program.to_account_info(),
            metadata_accounts,
            signer_seeds,
        ),
        data,
        false,  // is_mutable
        true,  // update_authority_is_signer
        None,  // collection_details
    )?;

    // Revoke mint authority (set to None)
    token::set_authority(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::SetAuthority {
                current_authority: launch_pool.to_account_info(),
                account_or_mint: ctx.accounts.token_mint.to_account_info(),
            },
            signer_seeds,
        ),
        token::spl_token::instruction::AuthorityType::MintTokens,
        None,
    )?;

    // Set status to Active
    launch_pool.status = LaunchStatus::Active;
    global_config.pool_count += 1;

    // Emit launch pool initialized event
    emit!(LaunchPoolInitialized {
        pool: launch_pool.key(),
        creator: creator.key(),
        token_mint: token_mint.key(),
        token_name: params.token_name,
        token_symbol: params.token_symbol,
        total_supply: TOTAL_SUPPLY,
        target_sol,
        duration,
        points_per_sol: launch_pool.points_per_sol,
        creator_lock_duration: lock_duration,
        start_time: launch_pool.start_time,
        end_time: launch_pool.end_time,
    });

    msg!("Launch pool initialized successfully");
    msg!("Token: {}", token_mint.key());
    msg!("Target: {} SOL", target_sol / LAMPORTS_PER_SOL);
    msg!("Duration: {} hours", duration / 3600);
    msg!("Creator lock duration: {} days", lock_duration / (24 * 3600));
    msg!("Creator linear unlock duration: {} days", linear_unlock_duration / (24 * 3600));

    Ok(())
}
