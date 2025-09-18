use solana_program::pubkey::Pubkey;

pub const SEEDS: &[&[u8]] = &[b"event_authority"];

/// Regenerate with `print_pda` helper below if the program ID changes.
pub const PDA: Pubkey = Pubkey::from_str_const("ADmPhmSFi6MHDTFTX2pB7x92WNuhNHXJWU438Lpz4KNe");

/// Regenerate with `print_pda` helper below if the program ID changes.
pub const BUMP: u8 = 255;

#[test]
/// Helper function to print the PDA for easy copy/paste into the const values above.
pub fn print_pda() {
    let (pda, bump) = solana_program::pubkey::Pubkey::find_program_address(
        &[b"event_authority"],
        &crate::PROGRAM_ID_PUBKEY,
    );
    println!("pda: {pda}\nbump: {bump}");
}

#[test]
pub fn check_pda() {
    assert_eq!(
        PDA,
        solana_program::pubkey::Pubkey::create_program_address(SEEDS, &crate::PROGRAM_ID_PUBKEY,)
            .expect("Should be OK")
    );
}
