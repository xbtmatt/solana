use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct MarketEscrow {
    trader: Pubkey,
    base: u64,
    quote: u64,
}
