use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Token, Transfer},
    token_interface::{TokenAccount, TokenInterface},
};

use crate::{const_pda::const_authority::{POOL_ID, VAULT_BUMP}, constants::{GLOBAL_CONFIG_SEED, LAUNCH_POOL_SEED, VAULT_AUTHORITY}, errors::LaunchpadError, state::{GlobalConfig, LaunchPool}};

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
        seeds = [LAUNCH_POOL_SEED, launch_pool.creator.as_ref(), &launch_pool.index.to_le_bytes()],
        bump = launch_pool.bump,
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

    /// Creator account from launch pool
    /// CHECK: verified against launch_pool.creator
    #[account(address = launch_pool.creator)]
    pub creator: UncheckedAccount<'info>,

    /// CHECK: pool address
    pub pool: UncheckedAccount<'info>,

    /// CHECK: position address
    #[account(mut)]
    pub position: UncheckedAccount<'info>,

    /// Treasury token a account
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_a_mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_a_program,
    )]
    pub treasury_token_a_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Treasury token b account
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_b_mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_b_program,
    )]
    pub treasury_token_b_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Creator token a account
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_a_mint,
        associated_token::authority = creator,
        associated_token::token_program = token_a_program,
    )]
    pub creator_token_a_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Creator token b account
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_b_mint,
        associated_token::authority = creator,
        associated_token::token_program = token_b_program,
    )]
    pub creator_token_b_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Vault authority token a account (receives fees from AMM)
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_a_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_a_program,
    )]
    pub vault_token_a_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Vault authority token b account (receives fees from AMM)
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = token_b_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_b_program,
    )]
    pub vault_token_b_account: Box<InterfaceAccount<'info, TokenAccount>>,

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

    /// CHECK: amm program event authority
    pub event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> ClaimPositionFee<'info> {
    pub fn claim_position_fee(&mut self) -> Result<()> {
        // Validate that the pool tokens match the launch pool tokens
        let token_a_mint = self.token_a_mint.key();
        let token_b_mint = self.token_b_mint.key();
        let launch_token_mint = self.launch_pool.token_mint;
        let launch_quote_mint = self.launch_pool.quote_mint;

        // Check if either token_a or token_b matches launch pool's token_mint
        // and the other matches quote_mint (SOL)
        let is_valid_pair =
            (token_a_mint == launch_token_mint && token_b_mint == launch_quote_mint) ||
            (token_b_mint == launch_token_mint && token_a_mint == launch_quote_mint);

        require!(
            is_valid_pair,
            LaunchpadError::InvalidTokenMint
        );

        let vault_authority_seeds: &[&[u8]] = &[VAULT_AUTHORITY, &[VAULT_BUMP]];

        // Step 1: Record the balances before claiming fees
        let token_a_before = self.vault_token_a_account.amount;
        let token_b_before = self.vault_token_b_account.amount;

        // Step 2: Claim fees from AMM to vault_authority's token accounts
        cp_amm::cpi::claim_position_fee(
            CpiContext::new_with_signer(
                self.amm_program.to_account_info(),
                cp_amm::cpi::accounts::ClaimPositionFeeCtx {
                    pool_authority: self.pool_authority.to_account_info(),
                    pool: self.pool.to_account_info(),
                    position: self.position.to_account_info(),
                    token_a_account: self.vault_token_a_account.to_account_info(),
                    token_b_account: self.vault_token_b_account.to_account_info(),
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

        // Step 3: Reload vault authority accounts to get the updated balances
        self.vault_token_a_account.reload()?;
        self.vault_token_b_account.reload()?;

        // Step 4: Calculate the actual fees claimed (difference between after and before)
        let token_a_after = self.vault_token_a_account.amount;
        let token_b_after = self.vault_token_b_account.amount;

        let token_a_claimed = token_a_after.saturating_sub(token_a_before);
        let token_b_claimed = token_b_after.saturating_sub(token_b_before);

        // Step 5: Calculate 50% of claimed fees for distribution
        let token_a_half = token_a_claimed / 2;
        let token_b_half = token_b_claimed / 2;

        // Step 6: Transfer 50% of token_a to treasury
        if token_a_half > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.vault_token_a_account.to_account_info(),
                        to: self.treasury_token_a_account.to_account_info(),
                        authority: self.vault_authority.to_account_info(),
                    },
                    &[&vault_authority_seeds[..]],
                ),
                token_a_half,
            )?;
        }

        // Step 7: Transfer 50% of token_a to creator
        if token_a_half > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.vault_token_a_account.to_account_info(),
                        to: self.creator_token_a_account.to_account_info(),
                        authority: self.vault_authority.to_account_info(),
                    },
                    &[&vault_authority_seeds[..]],
                ),
                token_a_claimed.checked_sub(token_a_half).ok_or(LaunchpadError::MathOverflow)?,
            )?;
        }

        // Step 8: Transfer 50% of token_b to treasury
        if token_b_half > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.vault_token_b_account.to_account_info(),
                        to: self.treasury_token_b_account.to_account_info(),
                        authority: self.vault_authority.to_account_info(),
                    },
                    &[&vault_authority_seeds[..]],
                ),
                token_b_claimed.checked_sub(token_b_half).ok_or(LaunchpadError::MathOverflow)?,
            )?;
        }

        // Step 9: Transfer 50% of token_b to creator
        if token_b_half > 0 {
            token::transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.vault_token_b_account.to_account_info(),
                        to: self.creator_token_b_account.to_account_info(),
                        authority: self.vault_authority.to_account_info(),
                    },
                    &[&vault_authority_seeds[..]],
                ),
                token_b_half,
            )?;
        }

        msg!("Fees claimed and distributed successfully");
        msg!("Token A claimed: {}, distributed: {} to treasury, {} to creator", token_a_claimed, token_a_half, token_a_half);
        msg!("Token B claimed: {}, distributed: {} to treasury, {} to creator", token_b_claimed, token_b_half, token_b_half);

        Ok(())
    }
}
