#![allow(unexpected_cfgs)]

use solana_program::entrypoint;

use processor::process_instruction;

pub mod instructions;
pub mod processor;
pub mod state;

// declare_id!("...address here...");

entrypoint!(process_instruction);
