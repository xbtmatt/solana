use crate::utils::{SectorIndex, Slab, NIL};
use bytemuck::{Pod, Zeroable};
use solana_program::{entrypoint::ProgramResult, program_error::ProgramError, pubkey::Pubkey};
use static_assertions::const_assert_eq;

pub const DEQUE_ACCOUNT_DISCRIMINANT: u64 = 0xf00dbabe;
pub const HEADER_FIXED_SIZE: usize = 96;

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable)]
pub struct DequeHeader {
    pub discriminant: u64,
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
    pub fn new_empty(deque_bump: u8, base_mint: &Pubkey, quote_mint: &Pubkey) -> Self {
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

    #[inline]
    pub fn verify_discriminant(&self) -> ProgramResult {
        if self.discriminant != DEQUE_ACCOUNT_DISCRIMINANT {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

const_assert_eq!(size_of::<DequeHeader>(), HEADER_FIXED_SIZE);
// Ensure the fixed size is exactly what's expected.
const_assert_eq!(
    HEADER_FIXED_SIZE,
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
