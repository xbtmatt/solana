use crate::utils::{SectorIndex, Slab, NIL};
use bytemuck::{Pod, Zeroable};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};
use static_assertions::const_assert_eq;

pub const DEQUE_ACCOUNT_DISCRIMINANT: u64 = 0xf00dbabe;
pub const HEADER_FIXED_SIZE: usize = 96;

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable)]
pub struct DequeHeader {
    pub discriminant: u64,
    pub version: u8,
    pub _padding: [u8; 3],
    pub len: SectorIndex,
    pub free_head: SectorIndex,
    pub deque_head: SectorIndex,
    pub deque_tail: SectorIndex,
    pub _padding2: [u8; 3],
    pub deque_bump: u8,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
}

unsafe impl Pod for DequeHeader {}

impl Slab for DequeHeader {}

impl DequeHeader {
    pub fn new_empty(deque_bump: u8, base_mint: &Pubkey, quote_mint: &Pubkey) -> Self {
        DequeHeader {
            discriminant: DEQUE_ACCOUNT_DISCRIMINANT,
            version: 0,
            _padding: [0; 3],
            len: 0,
            free_head: NIL,
            deque_head: NIL,
            deque_tail: NIL,
            _padding2: [0; 3],
            deque_bump,
            base_mint: *base_mint,
            quote_mint: *quote_mint,
        }
    }

    #[inline]
    pub fn verify_discriminant(&self) -> Result<(), ProgramError> {
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
    1 + // version
    3 + // padding
    4 + // len
    4 + // free_head
    4 + // deque_head
    4 + // deque_tail
    3 + // padding2
    1 + // deque_bump
    32 + // base_mint
    32 // quote_mint
);
const_assert_eq!(size_of::<DequeHeader>() % 8, 0);
