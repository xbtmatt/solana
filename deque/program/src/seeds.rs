pub mod event_authority {
    use solana_program::pubkey::Pubkey;

    pub const SEEDS: &[&[u8]] = &[b"event_authority", &[BUMP]];

    /// Regenerate with `print_pda` helper below if the program ID changes.
    pub const ID: Pubkey = Pubkey::from_str_const("ADmPhmSFi6MHDTFTX2pB7x92WNuhNHXJWU438Lpz4KNe");

    /// Regenerate with `print_pda` helper below if the program ID changes.
    pub const BUMP: u8 = 255;

    #[test]
    /// Helper function to print the PDA for easy copy/paste into the const values above.
    pub fn print_pda() {
        let (pda, bump) =
            solana_program::pubkey::Pubkey::find_program_address(&[b"event_authority"], &crate::ID);
        println!("pda: {pda}\nbump: {bump}");
    }

    #[test]
    pub fn check_pda() {
        assert_eq!(
            ID,
            solana_program::pubkey::Pubkey::create_program_address(SEEDS, &crate::ID)
                .expect("Should be OK")
        );
    }
}

pub mod market {
    use solana_program::pubkey::Pubkey;

    pub const MARKET_SEED_STR: &[u8] = b"market";

    pub fn find_market_address(base_mint: &Pubkey, quote_mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(crate::market_seeds!(base_mint, quote_mint), &crate::ID)
    }
}
