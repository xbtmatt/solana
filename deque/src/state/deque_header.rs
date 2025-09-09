use crate::{
    state::deque_old::Deque,
    utils::{Slab, SlotIndex, NIL},
};
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use solana_program::program_error::ProgramError;
use static_assertions::{const_assert, const_assert_eq};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum DequeAccount {
    FiveU64s(Deque<u64, 5>),
    TenU32s(Deque<u32, 10>),
}

pub const DEQUE_ACCOUNT_DISCRIMINANT: u64 = 0xf00dbabe;
pub const HEADER_FIXED_SIZE: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, Zeroable)]
pub struct DequeHeader {
    pub discriminant: u64,
    pub version: u8,
    pub _padding: [u8; 3],
    pub len: SlotIndex,
    pub free_head: SlotIndex,
    pub deque_head: SlotIndex,
    pub deque_tail: SlotIndex,
    _padding2: [u8; 4],
}

unsafe impl Pod for DequeHeader {}

impl Slab for DequeHeader {}

impl Default for DequeHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl DequeHeader {
    pub fn new() -> Self {
        DequeHeader {
            discriminant: DEQUE_ACCOUNT_DISCRIMINANT,
            version: 1,
            _padding: [0; 3],
            len: 0,
            free_head: NIL,
            deque_head: NIL,
            deque_tail: NIL,
            _padding2: [0; 4],
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

// Ensure the fixed size is exactly what's expected.
const_assert_eq!(
    size_of::<DequeHeader>(),
    8 + // discriminant
    1 + // version
    3 + // padding
    4 + // len
    4 + // free_head
    4 + // deque_head
    4 + // deque_tail
    4 // padding2
);
const_assert_eq!(size_of::<DequeHeader>(), HEADER_FIXED_SIZE);
const_assert!(size_of::<DequeHeader>().is_multiple_of(8));
