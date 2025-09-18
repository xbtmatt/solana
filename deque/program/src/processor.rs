use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instruction_enum::DequeInstruction;
use crate::instructions;
use crate::state::DequeInstruction;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let [discriminator, instruction_data @ ..] = instruction_data else {
        return Err(ProgramError::InvalidInstructionData);
    };

    let discriminator_enum = DequeInstruction::try_from(*discriminator)?;

    match discriminator_enum {
        DequeInstruction::Deposit => {
            instructions::deposit::process(program_id, accounts, instruction_data)?
        }
        _ => (),
    };

    let instruction = DequeInstruction::try_from_slice(instruction_data)?;

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
        DequeInstruction::FlushEventLog {} => Ok(()),
    }
}
