use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;

use crate::PROGRAM_ID_PUBKEY;

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct MarketEscrow {
    trader: Pubkey,
    base: u64,
    quote: u64,
}

#[macro_export]
macro_rules! deque_seeds {
    ( $base_mint:expr, $quote_mint:expr ) => {
        &[$base_mint.as_ref(), $quote_mint.as_ref(), b"deque"]
    };
}

#[macro_export]
macro_rules! deque_seeds_with_bump {
    ( $base_mint:expr, $quote_mint:expr, $bump:expr ) => {
        &[&[
            $base_mint.as_ref(),
            $quote_mint.as_ref(),
            b"deque",
            &[$bump],
        ]]
    };
}

#[macro_export]
macro_rules! vault_seeds {
    ( $vault:expr, $base_mint:expr, $quote_mint:expr ) => {
        &[
            $vault.as_ref(),
            $base_mint.as_ref(),
            $quote_mint.as_ref(),
            b"vault",
        ]
    };
}

#[macro_export]
macro_rules! vault_seeds_with_bump {
    ( $deque:expr, $base_mint:expr, $quote_mint:expr, $bump:expr) => {
        &[&[
            $deque.as_ref(),
            $base_mint.as_ref(),
            $quote_mint.as_ref(),
            b"vault",
            &[$bump],
        ]]
    };
}

/// Get the main storage/deque account and its associated bump.
pub fn get_deque_address(base_mint: &Pubkey, quote_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(deque_seeds!(base_mint, quote_mint), &PROGRAM_ID_PUBKEY)
}

/// Get the vault PDA and its associated bump.
pub fn get_vault_address(deque: &Pubkey, base_mint: &Pubkey, quote_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        vault_seeds!(deque, base_mint, quote_mint),
        &PROGRAM_ID_PUBKEY,
    )
}
