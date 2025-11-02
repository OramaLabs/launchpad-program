use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;

use crate::constants::VAULT_AUTHORITY;
use crate::errors::LaunchpadError;
use crate::events::LiquidityLocked;
use crate::state::{GlobalConfig, LaunchPool};
use crate::{cp_amm, const_pda::const_authority::VAULT_BUMP};

/// Lock liquidity in Meteora pool by calling cp_amm's permanent_lock_position
/// This can be called multiple times to progressively lock liquidity
/// Only admin can call this instruction
#[derive(Accounts)]
pub struct LockLiquidity<'info> {
    /// Global config account for admin verification
    #[account(
        seeds = [b"global_config"],
        bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// Launch pool account
    #[account(
        mut,
        constraint = launch_pool.position.is_some() @ LaunchpadError::InvalidPosition,
        constraint = launch_pool.position_nft_account.is_some() @ LaunchpadError::InvalidPositionNftAccount,
    )]
    pub launch_pool: Account<'info, LaunchPool>,

    /// Admin signer - must be the global config admin
    #[account(
        constraint = admin.key() == global_config.admin @ LaunchpadError::Unauthorized
    )]
    pub admin: Signer<'info>,

    /// CHECK: Meteora pool account (verified by cp_amm)
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,

    /// CHECK: Meteora position account (verified by cp_amm)
    #[account(
        mut,
        constraint = position.key() == launch_pool.position.unwrap() @ LaunchpadError::InvalidPosition
    )]
    pub position: UncheckedAccount<'info>,

    /// Position NFT token account
    #[account(
        constraint = position_nft_account.key() == launch_pool.position_nft_account.unwrap() @ LaunchpadError::InvalidPositionNftAccount,
        constraint = position_nft_account.amount == 1 @ LaunchpadError::InvalidPositionNftAccount,
        token::authority = vault_authority
    )]
    pub position_nft_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: Vault authority PDA
    #[account(
        seeds = [VAULT_AUTHORITY.as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// CHECK: cp_amm program
    #[account(address = cp_amm::ID)]
    pub amm_program: UncheckedAccount<'info>,

    /// CHECK: Meteora event authority
    pub damm_event_authority: UncheckedAccount<'info>,
}

impl<'info> LockLiquidity<'info> {
    /// Lock the specified amount of liquidity in the Meteora pool
    pub fn lock_liquidity(&mut self, liquidity_amount: u128) -> Result<()> {
        // Validate liquidity amount is greater than 0
        require!(
            liquidity_amount > 0,
            LaunchpadError::InvalidAmount
        );

        // Prepare PDA signer seeds
        let signer_seeds: &[&[&[u8]]] = &[&[VAULT_AUTHORITY, &[VAULT_BUMP]]];

        // Call cp_amm's permanent_lock_position via CPI
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
            liquidity_amount,
        )?;

        // Emit event
        let clock = Clock::get()?;
        emit!(LiquidityLocked {
            launch_pool: self.launch_pool.key(),
            position: self.position.key(),
            pool: self.pool.key(),
            locked_amount: liquidity_amount,
            admin: self.admin.key(),
            timestamp: clock.unix_timestamp,
        });

        msg!("Successfully locked {} liquidity units", liquidity_amount);

        Ok(())
    }
}

/// Handler function for lock_liquidity instruction
pub fn handle_lock_liquidity(ctx: Context<LockLiquidity>, liquidity_amount: u128) -> Result<()> {
    ctx.accounts.lock_liquidity(liquidity_amount)
}
