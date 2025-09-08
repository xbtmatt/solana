#![allow(unexpected_cfgs)]

use solana_program::{declare_id, entrypoint, msg, pubkey::Pubkey};

use processor::process_instruction;

pub mod instructions;
pub mod processor;
pub mod state;
pub mod tests;

pub const PROGRAM_ID_STR: &str = "2XMZLhL2aL95mjmEC3t8ocKKiku7MLM7CnqUBvci44F4";
pub const PROGRAM_ID_PUBKEY: Pubkey = Pubkey::from_str_const(PROGRAM_ID_STR);

pub fn log_bytes(bytes: &[u8]) {
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    msg!(&hex);
}

declare_id!(PROGRAM_ID_STR);

entrypoint!(process_instruction);
