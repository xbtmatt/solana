use borsh::{BorshDeserialize, BorshSerialize};

use crate::utils::SlotIndex;

#[derive(BorshSerialize, BorshDeserialize)]
pub enum DequeInstruction {
    Initialize { deque_type: u8 },
    PushFront { value: Vec<u8> },
    PushBack { value: Vec<u8> },
    Remove { index: SlotIndex },
}
