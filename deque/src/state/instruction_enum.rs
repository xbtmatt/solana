use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::utils::SectorIndex;

#[derive(BorshSerialize, BorshDeserialize)]
pub enum DequeInstruction {
    Initialize {
        deque_type: u8,
        num_sectors: u16,
        base_mint: Pubkey,
        quote_mint: Pubkey,
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
    },
    Withdraw {
        amount: u64,
    },
}
