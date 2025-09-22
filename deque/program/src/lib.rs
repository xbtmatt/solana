#![allow(unexpected_cfgs)]

use solana_program::{declare_id, entrypoint};

use processor::process_instruction;

pub mod context;
pub mod events;
pub mod instruction_enum;
pub mod instructions;
pub mod macros;
pub mod pack;
pub mod processor;
pub mod seeds;
pub mod shared;
pub mod state;
pub(crate) mod syscalls;
pub mod utils;
pub mod validation;

declare_id!("4o8MdmWKP5FzAacZsj4QboX7rkUQbmMp87MBwdqarHtb");

entrypoint!(process_instruction);
