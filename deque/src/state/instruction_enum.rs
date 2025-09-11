use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum MarketEscrowChoice {
    Base,
    Quote,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum DequeInstruction {
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
}
