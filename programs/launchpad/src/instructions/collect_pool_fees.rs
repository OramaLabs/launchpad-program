use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::Token,
    token_interface::{TokenAccount, TokenInterface},
};

use crate::{const_pda::const_authority::{POOL_ID, VAULT_BUMP}, constants::{GLOBAL_CONFIG_SEED, VAULT_AUTHORITY}, errors::LaunchpadError, state::{GlobalConfig, LaunchPool}};

#[derive(Accounts)]
pub struct ClaimPositionFee<'info> {
    /// CHECK: pool authority
    #[account(
        mut,
        address = POOL_ID,
    )]
    pub pool_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = launch_pool.is_migrated() @ LaunchpadError::NotMigrated,
    )]
    pub launch_pool: Box<Account<'info, LaunchPool>>,

    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
    )]
    pub global_config: Box<Account<'info, GlobalConfig>>,

    /// CHECK: vault authority
    #[account(
        mut,
        seeds = [VAULT_AUTHORITY.as_ref()],
        bump,
    )]
    pub vault_authority: SystemAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: owner of the propposal
    #[account(address = global_config.admin.key())]
    pub treasury: UncheckedAccount<'info>,

    /// CHECK: pool address
    pub pool: UncheckedAccount<'info>,

    /// CHECK: position address
    #[account(mut)]
    pub position: UncheckedAccount<'info>,

    /// The user token a account
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_a_mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_a_program,
    )]
    pub token_a_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The user token b account
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_b_mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_b_program,
    )]
    pub token_b_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The vault token account for input token
    #[account(mut, token::token_program = token_a_program, token::mint = token_a_mint)]
    pub token_a_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The vault token account for output token
    #[account(mut, token::token_program = token_b_program, token::mint = token_b_mint)]
    pub token_b_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK:
    pub token_a_mint: UncheckedAccount<'info>,

    /// CHECK:
    pub token_b_mint: UncheckedAccount<'info>,

    /// CHECK:
    pub position_nft_account: UncheckedAccount<'info>,

    pub token_a_program: Interface<'info, TokenInterface>,

    pub token_b_program: Interface<'info, TokenInterface>,

    /// CHECK: amm program address
    #[account(address = cp_amm::ID)]
    pub amm_program: UncheckedAccount<'info>,

    /// CHECK:
    pub event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> ClaimPositionFee<'info> {
    pub fn claim_position_fee(&mut self) -> Result<()> {
        let vault_authority_seeds: &[&[u8]] = &[VAULT_AUTHORITY, &[VAULT_BUMP]];

        cp_amm::cpi::claim_position_fee(
            CpiContext::new_with_signer(
                self.amm_program.to_account_info(),
                cp_amm::cpi::accounts::ClaimPositionFeeCtx {
                    pool_authority: self.pool_authority.to_account_info(),
                    pool: self.pool.to_account_info(),
                    position: self.position.to_account_info(),
                    token_a_account: self.token_a_account.to_account_info(),
                    token_b_account: self.token_b_account.to_account_info(),
                    token_a_vault: self.token_a_vault.to_account_info(),
                    token_b_vault: self.token_b_vault.to_account_info(),
                    token_a_mint: self.token_a_mint.to_account_info(),
                    token_b_mint: self.token_b_mint.to_account_info(),
                    position_nft_account: self.position_nft_account.to_account_info(),
                    owner: self.vault_authority.to_account_info(),
                    token_a_program: self.token_a_program.to_account_info(),
                    token_b_program: self.token_b_program.to_account_info(),
                    event_authority: self.event_authority.to_account_info(),
                    program: self.amm_program.to_account_info(),
                },
                &[&vault_authority_seeds[..]],
            )
        )?;

        self.token_a_account.reload()?;
        self.token_b_account.reload()?;

        Ok(())
    }
}
