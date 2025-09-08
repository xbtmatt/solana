use crate::state::deque::Deque;
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum DequeAccount {
    FiveU64s(Deque<u64, 5>),
    TenU32s(Deque<u32, 10>),
}
