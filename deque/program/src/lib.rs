#![allow(unexpected_cfgs)]

use solana_program::{declare_id, entrypoint, pubkey::Pubkey};

use processor::process_instruction;

pub mod context;
pub mod instructions;
pub mod processor;
pub mod shared;
pub mod state;
pub mod utils;
pub mod validation;

pub const PROGRAM_ID_STR: &str = "9SM4HUDDWsKDs9wCkfdGwkfDtUL9WwXUnmqdwNnZTzBW";
pub const PROGRAM_ID_PUBKEY: Pubkey = Pubkey::from_str_const(PROGRAM_ID_STR);

declare_id!(PROGRAM_ID_STR);

entrypoint!(process_instruction);
