pub mod const_authority {
    use anchor_lang::solana_program::pubkey::Pubkey;
    use const_crypto::ed25519;
    use cp_amm::constants::seeds::POOL_AUTHORITY_PREFIX;
    use crate::constants::VAULT_AUTHORITY;

    const POOL_AUTHORITY_AND_BUMP: ([u8; 32], u8) = ed25519::derive_program_address(
        &[POOL_AUTHORITY_PREFIX],
        &cp_amm::ID_CONST.to_bytes(),
    );

    pub const POOL_ID: Pubkey = Pubkey::new_from_array(POOL_AUTHORITY_AND_BUMP.0);

    const VAULT_AUTHORITY_AND_BUMP: ([u8; 32], u8) = ed25519::derive_program_address(
        &[VAULT_AUTHORITY],
        &crate::ID_CONST.to_bytes(),
    );

    pub const VAULT_BUMP: u8 = VAULT_AUTHORITY_AND_BUMP.1;
}
