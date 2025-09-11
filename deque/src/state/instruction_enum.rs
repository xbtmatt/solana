use borsh::{BorshDeserialize, BorshSerialize};

use crate::utils::SectorIndex;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum MarketEscrowChoice {
    Base,
    Quote,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum DequeInstruction {
    Initialize {
        deque_type: u8,
        num_sectors: u16,
    },
    PushFront {
        value: Vec<u8>,
    },
    PushBack {
        value: Vec<u8>,
    },
    Remove {
        index: SectorIndex,
    },
    Resize {
        num_sectors: u16,
    },
    Deposit {
        amount: u64,
        choice: MarketEscrowChoice,
    },
    Withdraw {
        amount: u64,
        choice: MarketEscrowChoice,
    },
}
