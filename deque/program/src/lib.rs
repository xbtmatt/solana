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

#[cfg(not(target_os = "solana"))]
pub const fn id_str() -> &'static str {
    "44w6cQa6hhEqsbfokN38qTXeo2JFozX6SRL9ChZDSnSW"
}
declare_id!("44w6cQa6hhEqsbfokN38qTXeo2JFozX6SRL9ChZDSnSW");

entrypoint!(process_instruction);
