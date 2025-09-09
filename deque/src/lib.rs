#![allow(unexpected_cfgs)]
#![feature(generic_const_exprs)]

use solana_program::{declare_id, entrypoint, pubkey::Pubkey};

use processor::process_instruction;

pub mod instructions;
pub mod processor;
pub mod state;
pub mod tests;
pub mod utils;

pub const PROGRAM_ID_STR: &str = "2XMZLhL2aL95mjmEC3t8ocKKiku7MLM7CnqUBvci44F4";
pub const PROGRAM_ID_PUBKEY: Pubkey = Pubkey::from_str_const(PROGRAM_ID_STR);

declare_id!(PROGRAM_ID_STR);

entrypoint!(process_instruction);
