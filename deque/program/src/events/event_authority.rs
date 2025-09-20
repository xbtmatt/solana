use solana_program::pubkey::Pubkey;

pub const SEEDS: &[&[u8]] = &[b"event_authority", &[BUMP]];

/// Regenerate with `print_pda` helper below if the program ID changes.
pub const ID: Pubkey = Pubkey::from_str_const("cwEgVDNTb5vMaQWuUcNpr9D3ZXLFYnSCFE6Zkt5FDSN");

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
        ID,
        solana_program::pubkey::Pubkey::create_program_address(SEEDS, &crate::PROGRAM_ID_PUBKEY)
            .expect("Should be OK")
    );
}
