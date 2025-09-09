use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{TokenAccount, TokenInterface},
};
use cp_amm::state::Config;
use std::u64;

use crate::{const_pda::const_authority::{POOL_ID, VAULT_BUMP}, constants::TOKEN_VAULT};
use crate::constants::{ VAULT_AUTHORITY };
use crate::errors::LaunchpadError;
use crate::state::{LaunchPool, LaunchStatus};
use crate::utils::get_liquidity_for_adding_liquidity;

#[derive(Accounts)]
pub struct DammV2<'info> {
    #[account(
        mut,
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
        seeds = [TOKEN_VAULT, vault_authority.key().as_ref(), base_mint.key().as_ref()],
        bump,
        token::mint = base_mint,
        token::authority = vault_authority,
        token::token_program = token_base_program,
      )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [TOKEN_VAULT, vault_authority.key().as_ref(), quote_mint.key().as_ref()],
        bump,
        token::mint = quote_mint,
        token::authority = vault_authority,
        token::token_program = token_quote_program
    )]
    pub wsol_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    /// CHECK: pool authority
    #[account(
        mut,
        address = POOL_ID,
    )]
    pub pool_authority: AccountInfo<'info>,
    /// CHECK: pool config
    pool_config: AccountLoader<'info, Config>,
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
    /// CHECK: payer
    #[account(mut)]
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
    pub fn create_pool(&mut self, sqrt_price: u128) -> Result<()> {
        let launch_pool = &mut self.launch_pool;

        // Verify launch pool is in correct state
        require!(
            launch_pool.status == LaunchStatus::Success,
            LaunchpadError::InvalidLaunchStatus
        );

        // Verify we have sufficient liquidity to create pool
        require!(
            launch_pool.liquidity_allocation > 0 && launch_pool.liquidity_sol > 0,
            LaunchpadError::InsufficientLiquidity
        );

        let signer_seeds: &[&[&[u8]]] = &[&[VAULT_AUTHORITY, &[VAULT_BUMP]]];

        let config = self.pool_config.load()?;
        let base_amount: u64 = launch_pool.liquidity_allocation;
        let quote_amount: u64 = launch_pool.liquidity_sol;

        let liquidity = get_liquidity_for_adding_liquidity(
            base_amount,
            quote_amount,
            sqrt_price,
            config.sqrt_min_price,
            config.sqrt_max_price,
        )?;

        cp_amm::cpi::initialize_pool(
            CpiContext::new_with_signer(
                self.amm_program.to_account_info(),
                cp_amm::cpi::accounts::InitializePoolCtx {
                    creator: self.vault_authority.to_account_info(),
                    position_nft_mint: self.position_nft_mint.to_account_info(),
                    position_nft_account: self.position_nft_account.to_account_info(),
                    payer: self.vault_authority.to_account_info(),
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
            cp_amm::InitializePoolParameters {
                liquidity,
                sqrt_price,
                activation_point: None,
            },
        )?;

        // cp_amm::cpi::permanent_lock_position(
        //     CpiContext::new_with_signer(
        //         self.amm_program.to_account_info(),
        //         cp_amm::cpi::accounts::PermanentLockPositionCtx {
        //             pool: self.pool.to_account_info(),
        //             position: self.position.to_account_info(),
        //             position_nft_account: self.position_nft_account.to_account_info(),
        //             owner: self.pool_authority.to_account_info(),
        //             event_authority: self.damm_event_authority.to_account_info(),
        //             program: self.amm_program.to_account_info(),
        //         },
        //         signer_seeds,
        //     ),
        //     liquidity,
        // )?;

        // 设置创建者代币解锁开始时间
        let clock = Clock::get()?;
        launch_pool.creator_unlock_start_time = clock.unix_timestamp;

        launch_pool.status = LaunchStatus::Migrated;

        msg!("Creator token unlock will start at: {}", clock.unix_timestamp);
        msg!("Lock duration: {} days", launch_pool.creator_lock_duration / (24 * 3600));
        msg!("Linear unlock duration: {} days", launch_pool.creator_linear_unlock_duration / (24 * 3600));

        Ok(())
    }
}
