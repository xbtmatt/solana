use borsh::{BorshDeserialize, BorshSerialize};

use crate::utils::SlotIndex;

#[derive(BorshSerialize, BorshDeserialize)]
pub enum DequeInstruction {
    Initialize { deque_type: u8, num_slots: u16 },
    PushFront { value: Vec<u8> },
    PushBack { value: Vec<u8> },
    Remove { index: SlotIndex },
    Resize { num_slots: u16 },
}
