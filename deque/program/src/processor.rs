use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instruction_enum::DequeInstruction;
use crate::instructions;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = DequeInstruction::unpack(instruction_data)?;

    match instruction {
        DequeInstruction::Initialize { num_sectors } => {
            instructions::initialize::process(program_id, accounts, num_sectors)
        }
        DequeInstruction::Resize { num_sectors } => {
            instructions::resize::process(program_id, accounts, num_sectors)
        }
        DequeInstruction::Deposit { amount, choice } => {
            instructions::deposit::process(program_id, accounts, amount, choice)
        }
        DequeInstruction::Withdraw { choice } => {
            instructions::withdraw::process(program_id, accounts, choice)
        }
        DequeInstruction::FlushEventLog => Ok(()),
    }
}
