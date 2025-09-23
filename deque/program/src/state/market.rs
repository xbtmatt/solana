use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;

use crate::instruction_enum::MarketChoice;

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct MarketEscrow {
    pub trader: Pubkey,
    pub base: u64,
    pub quote: u64,
}

impl MarketEscrow {
    #[inline(always)]
    pub fn new(trader: Pubkey, base: u64, quote: u64) -> Self {
        MarketEscrow {
            trader,
            base,
            quote,
        }
    }

    #[inline(always)]
    pub fn amount_from_choice(&self, choice: &MarketChoice) -> u64 {
        match choice {
            MarketChoice::Base => self.base,
            MarketChoice::Quote => self.quote,
        }
    }

    #[inline(always)]
    pub fn amount_of_opposite_choice(&self, choice: &MarketChoice) -> u64 {
        match choice {
            MarketChoice::Base => self.quote,
            MarketChoice::Quote => self.base,
        }
    }
}
