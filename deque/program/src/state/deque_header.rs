use crate::{
    shared::error::DequeError,
    utils::{SectorIndex, Slab, NIL},
};
use bytemuck::{Pod, Zeroable};
use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};
use static_assertions::const_assert_eq;

pub const DEQUE_ACCOUNT_DISCRIMINANT: [u8; 8] = 0xd00d00b00b00f00du64.to_le_bytes();
pub const DEQUE_HEADER_SIZE: usize = 96;

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable)]
pub struct DequeHeader {
    pub discriminant: [u8; 8],
    pub len: SectorIndex,
    pub free_head: SectorIndex,
    pub deque_head: SectorIndex,
    pub deque_tail: SectorIndex,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub version: u8,
    pub deque_bump: u8,
    // Explicitly mark the padding that repr(C) will add implicitly.
    pub _padding: [u8; 6],
}

unsafe impl Pod for DequeHeader {}

impl Slab for DequeHeader {}

impl DequeHeader {
    pub fn init(deque_bump: u8, base_mint: &Pubkey, quote_mint: &Pubkey) -> Self {
        DequeHeader {
            discriminant: DEQUE_ACCOUNT_DISCRIMINANT,
            base_mint: *base_mint,
            quote_mint: *quote_mint,
            len: 0,
            free_head: NIL,
            deque_head: NIL,
            deque_tail: NIL,
            version: 0,
            deque_bump,
            _padding: [0; 6],
        }
    }

    #[inline(always)]
    pub fn verify_discriminant(&self) -> ProgramResult {
        if self.discriminant != DEQUE_ACCOUNT_DISCRIMINANT {
            return Err(DequeError::InvalidDiscriminant.into());
        }
        Ok(())
    }
}

const_assert_eq!(size_of::<DequeHeader>(), DEQUE_HEADER_SIZE);
// Ensure the fixed size is exactly what's expected.
const_assert_eq!(
    DEQUE_HEADER_SIZE,
    8 + // discriminant
    4 + // len
    4 + // free_head
    4 + // deque_head
    4 + // deque_tail
    32 + // base_mint
    32 + // quote_mint
    1 + // version
    1 + // deque_bump
    6 // _padding
);
