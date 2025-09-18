use borsh::{BorshDeserialize, BorshSerialize};

use crate::state::DequeInstruction;

pub mod event_emitter;

#[derive(Debug, Copy, Clone, BorshSerialize, BorshDeserialize)]
pub enum EmittableEvent {
    DequeInstruction::Initialize,
}
