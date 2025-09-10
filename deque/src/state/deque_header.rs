use crate::{
    state::{DequeNode, StackNode},
    utils::{Slab, SlotIndex, NIL},
};
use bytemuck::{Pod, Zeroable};
use solana_program::program_error::ProgramError;
use static_assertions::const_assert_eq;

pub const DEQUE_ACCOUNT_DISCRIMINANT: u64 = 0xf00dbabe;
pub const HEADER_FIXED_SIZE: usize = 32;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable)]
pub enum DequeType {
    U32,
    U64,
}

const_assert_eq!(size_of::<DequeNode<u32>>(), size_of::<StackNode<u32>>());
const_assert_eq!(size_of::<DequeNode<u64>>(), size_of::<StackNode<u64>>());

impl TryFrom<u8> for DequeType {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::U32),
            1 => Ok(Self::U64),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

impl From<DequeType> for u8 {
    fn from(dt: DequeType) -> u8 {
        dt as u8
    }
}

impl DequeType {
    #[inline(always)]
    pub fn elem_size(self) -> usize {
        match self {
            DequeType::U32 => 4,
            DequeType::U64 => 8,
        }
    }

    #[inline(always)]
    pub fn slot_size(self) -> usize {
        match self {
            DequeType::U32 => size_of::<DequeNode<u32>>(),
            DequeType::U64 => size_of::<DequeNode<u64>>(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable)]
pub struct DequeHeader {
    pub discriminant: u64,
    pub version: u8,
    pub deque_type: u8,
    pub _padding: [u8; 2],
    pub len: SlotIndex,
    pub free_head: SlotIndex,
    pub deque_head: SlotIndex,
    pub deque_tail: SlotIndex,
    pub _padding2: [u8; 4],
}

unsafe impl Pod for DequeHeader {}

impl Slab for DequeHeader {}

impl DequeHeader {
    pub fn new_empty(deque_type: DequeType) -> Self {
        DequeHeader {
            discriminant: DEQUE_ACCOUNT_DISCRIMINANT,
            deque_type: deque_type.into(),
            version: 0,
            _padding: [0; 2],
            len: 0,
            free_head: NIL,
            deque_head: NIL,
            deque_tail: NIL,
            _padding2: [0; 4],
        }
    }

    pub fn get_type(&self) -> DequeType {
        self.deque_type
            .try_into()
            .expect("Deque type should have already been validated.")
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
    1 + // deque type
    2 + // padding
    4 + // len
    4 + // free_head
    4 + // deque_head
    4 + // deque_tail
    4 // padding2
);
const_assert_eq!(size_of::<DequeHeader>() % 8, 0);
