use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub enum MarketEscrowChoice {
    Base,
    Quote,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum DequeInstructionBorsh {
    Initialize {
        num_sectors: u16,
    },
    Resize {
        num_sectors: u16,
    },
    Deposit {
        amount: u64,
        choice: MarketEscrowChoice,
    },
    Withdraw {
        choice: MarketEscrowChoice,
    },
    FlushEventLog {},
}
