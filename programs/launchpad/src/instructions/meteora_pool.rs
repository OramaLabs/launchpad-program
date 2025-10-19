use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{TokenAccount, TokenInterface},
};
use cp_amm::types::{
    BaseFeeParameters, InitializeCustomizablePoolParameters, PoolFeeParameters,
};
use std::u64;

use crate::{const_pda::const_authority::VAULT_BUMP, constants::{FEE_DENOMINATOR, MAX_BASIS_POINT, MAX_SQRT_PRICE, MIN_SQRT_PRICE, SQRT_PRICE, TOKEN_VAULT}, cp_amm, state::GlobalConfig};
use crate::constants::{LAUNCH_POOL_SEED, VAULT_AUTHORITY};
use crate::errors::LaunchpadError;
use crate::state::{LaunchPool, LaunchStatus};
use crate::utils::{get_liquidity_for_adding_liquidity};

#[derive(Accounts)]
pub struct DammV2<'info> {
    /// Global config account for admin verification
    #[account(
        seeds = [b"global_config"],
        bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        seeds = [LAUNCH_POOL_SEED, launch_pool.creator.as_ref(), &launch_pool.index.to_le_bytes()],
        bump = launch_pool.bump,
        constraint = launch_pool.is_success() @ LaunchpadError::LaunchFailed,
    )]
    pub launch_pool: Box<Account<'info, LaunchPool>>,

    /// CHECK: vault authority
    #[account(
        mut,
        seeds = [VAULT_AUTHORITY.as_ref()],
        bump,
    )]
    pub vault_authority: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [TOKEN_VAULT, launch_pool.key().as_ref(), vault_authority.key().as_ref(), base_mint.key().as_ref()],
        bump,
        token::mint = base_mint,
        token::authority = vault_authority,
        token::token_program = token_base_program,
      )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [TOKEN_VAULT, launch_pool.key().as_ref(), vault_authority.key().as_ref(), quote_mint.key().as_ref()],
        bump,
        token::mint = quote_mint,
        token::authority = vault_authority,
        token::token_program = token_quote_program
    )]
    pub wsol_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    /// CHECK: pool config (cp_amm Config)
    pub pool_config: UncheckedAccount<'info>,
    /// CHECK: pool
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    /// CHECK: position nft mint for partner
    #[account(mut, signer)]
    pub position_nft_mint: UncheckedAccount<'info>,
    /// CHECK: damm pool authority
    pub damm_pool_authority: UncheckedAccount<'info>,
    /// CHECK: position nft account for partner
    #[account(mut)]
    pub position_nft_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = cp_amm::ID)]
    pub amm_program: UncheckedAccount<'info>,
    /// CHECK: base token mint
    #[account(
        mut,
        constraint = base_mint.key() == launch_pool.token_mint @ LaunchpadError::InvalidTokenMint
    )]
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: quote token mint
    #[account(
        mut,
        constraint = quote_mint.key() == launch_pool.quote_mint @ LaunchpadError::InvalidQuoteMint
    )]
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub token_a_vault: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub token_b_vault: UncheckedAccount<'info>,
    /// CHECK: payer - must be either launch pool creator or global config admin
    #[account(
        mut,
        constraint = payer.key() == launch_pool.creator || payer.key() == global_config.admin @ LaunchpadError::Unauthorized
    )]
    pub payer: Signer<'info>,
    /// CHECK: token_program
    pub token_base_program: Interface<'info, TokenInterface>,
    /// CHECK: token_program
    pub token_quote_program: Interface<'info, TokenInterface>,
    /// CHECK: token_program
    pub token_2022_program: Interface<'info, TokenInterface>,
    /// CHECK: damm event authority
    pub damm_event_authority: UncheckedAccount<'info>,
    /// System program.
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> DammV2<'info> {
    fn initialize_pool(&mut self) -> Result<()> {
        let base_amount: u64 = self.launch_pool.liquidity_allocation;
        let quote_amount: u64 = self.launch_pool.liquidity_sol;

        // Calculate fair sqrt_price based on actual token amounts
        let sqrt_price = SQRT_PRICE;

        // Validate calculated sqrt_price is within reasonable bounds
        require!(
            sqrt_price >= MIN_SQRT_PRICE && sqrt_price <= MAX_SQRT_PRICE,
            LaunchpadError::InvalidAmount
        );

        let liquidity = get_liquidity_for_adding_liquidity(
            base_amount,
            quote_amount,
            sqrt_price,
            MIN_SQRT_PRICE,
            MAX_SQRT_PRICE,
        )?;

        // Calculate 1.5% fee numerator
        // 1.5% = 150 BPS
        // numerator = 150 * FEE_DENOMINATOR / MAX_BASIS_POINT
        let base_fee_numerator = (150u128 * FEE_DENOMINATOR as u128 / MAX_BASIS_POINT as u128) as u64;

        // Create fee parameters
        let base_fee = BaseFeeParameters {
            cliff_fee_numerator: base_fee_numerator,
            ..Default::default()
        };

        let pool_fees = PoolFeeParameters {
            base_fee,
            protocol_fee_percent: 0,
            partner_fee_percent: 0,
            referral_fee_percent: 0,
            dynamic_fee: None, // Fixed fee, no dynamic fee
        };

        // Create initialization parameters
        let initialize_pool_params = InitializeCustomizablePoolParameters {
            pool_fees,
            sqrt_min_price: MIN_SQRT_PRICE,
            sqrt_max_price: MAX_SQRT_PRICE,
            has_alpha_vault: false,
            liquidity,
            sqrt_price,
            activation_type: 1, // timestamp
            collect_fee_mode: 0, // default mode
            activation_point: None,
        };

        let signer_seeds: &[&[&[u8]]] = &[&[VAULT_AUTHORITY, &[VAULT_BUMP]]];
        cp_amm::cpi::initialize_pool_with_dynamic_config(
            CpiContext::new_with_signer(
                self.amm_program.to_account_info(),
                cp_amm::cpi::accounts::InitializePoolWithDynamicConfig {
                    creator: self.vault_authority.to_account_info(),
                    position_nft_mint: self.position_nft_mint.to_account_info(),
                    position_nft_account: self.position_nft_account.to_account_info(),
                    payer: self.vault_authority.to_account_info(),
                    pool_creator_authority: self.vault_authority.to_account_info(),
                    config: self.pool_config.to_account_info(),
                    pool_authority: self.damm_pool_authority.to_account_info(),
                    pool: self.pool.to_account_info(),
                    position: self.position.to_account_info(),
                    token_a_mint: self.base_mint.to_account_info(),
                    token_b_mint: self.quote_mint.to_account_info(),
                    token_a_vault: self.token_a_vault.to_account_info(),
                    token_b_vault: self.token_b_vault.to_account_info(),
                    payer_token_a: self.token_vault.to_account_info(),
                    payer_token_b: self.wsol_vault.to_account_info(),
                    token_a_program: self.token_base_program.to_account_info(),
                    token_b_program: self.token_quote_program.to_account_info(),
                    token_2022_program: self.token_2022_program.to_account_info(),
                    system_program: self.system_program.to_account_info(),
                    event_authority: self.damm_event_authority.to_account_info(),
                    program: self.amm_program.to_account_info(),
                },
                signer_seeds,
            ),
            initialize_pool_params,
        )?;

        cp_amm::cpi::permanent_lock_position(
            CpiContext::new_with_signer(
                self.amm_program.to_account_info(),
                cp_amm::cpi::accounts::PermanentLockPosition {
                    pool: self.pool.to_account_info(),
                    position: self.position.to_account_info(),
                    position_nft_account: self.position_nft_account.to_account_info(),
                    owner: self.vault_authority.to_account_info(),
                    event_authority: self.damm_event_authority.to_account_info(),
                    program: self.amm_program.to_account_info(),
                },
                signer_seeds,
            ),
            liquidity/2,
        )?;

        Ok(())
    }

    pub fn create_pool(&mut self) -> Result<()> {
        // Verify launch pool is in correct state
        require!(
            self.launch_pool.status == LaunchStatus::Success,
            LaunchpadError::InvalidLaunchStatus
        );

        // Verify we have sufficient liquidity to create pool
        require!(
            self.launch_pool.liquidity_allocation > 0 && self.launch_pool.liquidity_sol > 0,
            LaunchpadError::InsufficientLiquidity
        );

        // Record vault balances before initialize_pool
        let token_vault_before = self.token_vault.amount;
        let wsol_vault_before = self.wsol_vault.amount;

        // Extract values needed after initialize_pool
        let raised_sol = self.launch_pool.raised_sol;
        let total_supply = self.launch_pool.total_supply;
        let creator_allocation = self.launch_pool.creator_allocation;

        msg!("Vault balances before initialize_pool:");
        msg!("Token vault: {}", token_vault_before);
        msg!("WSOL vault: {}", wsol_vault_before);

        self.initialize_pool()?;

        // Reload accounts to get updated balances
        self.token_vault.reload()?;
        self.wsol_vault.reload()?;

        // Record vault balances after initialize_pool
        let token_vault_after = self.token_vault.amount;
        let wsol_vault_after = self.wsol_vault.amount;

        msg!("Vault balances after initialize_pool:");
        msg!("Token vault: {}", token_vault_after);
        msg!("WSOL vault: {}", wsol_vault_after);

        // Calculate actual amounts used
        let actual_token_used = token_vault_before.saturating_sub(token_vault_after);
        let actual_sol_used = wsol_vault_before.saturating_sub(wsol_vault_after);

        msg!("Actual amounts used for liquidity:");
        msg!("Tokens used: {}", actual_token_used);
        msg!("SOL used: {}", actual_sol_used);

        // Update launch_pool based on actual usage
        // 1. Update liquidity_sol and excess_sol
        self.launch_pool.liquidity_sol = actual_sol_used;
        self.launch_pool.excess_sol = raised_sol.checked_sub(actual_sol_used)
            .ok_or(LaunchpadError::MathOverflow)?;

        // 2. Update sale_allocation and liquidity_allocation
        self.launch_pool.liquidity_allocation = actual_token_used;
        self.launch_pool.sale_allocation = total_supply
            .checked_sub(creator_allocation)
            .ok_or(LaunchpadError::MathOverflow)?
            .checked_sub(actual_token_used)
            .ok_or(LaunchpadError::MathOverflow)?;

        msg!("Updated launch_pool allocations:");
        msg!("liquidity_sol: {}", self.launch_pool.liquidity_sol);
        msg!("excess_sol: {}", self.launch_pool.excess_sol);
        msg!("liquidity_allocation: {}", self.launch_pool.liquidity_allocation);
        msg!("sale_allocation: {}", self.launch_pool.sale_allocation);

        let clock = Clock::get()?;
        self.launch_pool.creator_unlock_start_time = clock.unix_timestamp;

        // Set Meteora position fields after successful migration
        self.launch_pool.position = Some(self.position.key());
        self.launch_pool.position_nft_account = Some(self.position_nft_account.key());

        self.launch_pool.status = LaunchStatus::Migrated;

        msg!("Creator token unlock will start at: {}", clock.unix_timestamp);
        msg!("Lock duration: {} days", self.launch_pool.creator_lock_duration / (24 * 3600));
        msg!("Linear unlock duration: {} days", self.launch_pool.creator_linear_unlock_duration / (24 * 3600));

        Ok(())
    }
}
