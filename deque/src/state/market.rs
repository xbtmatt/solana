use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;

use crate::{state::MarketEscrowChoice, PROGRAM_ID_PUBKEY};

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct MarketEscrow {
    pub trader: Pubkey,
    pub base: u64,
    pub quote: u64,
}

impl MarketEscrow {
    pub fn new(trader: Pubkey, base: u64, quote: u64) -> Self {
        MarketEscrow {
            trader,
            base,
            quote,
        }
    }

    pub fn amount_from_choice(&self, choice: MarketEscrowChoice) -> u64 {
        match choice {
            MarketEscrowChoice::Base => self.base,
            MarketEscrowChoice::Quote => self.quote,
        }
    }
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
    ( $deque:expr, $base_mint:expr, $quote_mint:expr ) => {
        &[
            $deque.as_ref(),
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
