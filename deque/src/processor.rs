use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instructions;
use crate::state::DequeInstruction;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = DequeInstruction::try_from_slice(instruction_data)?;

    match instruction {
        DequeInstruction::Initialize {
            deque_type,
            num_sectors,
        } => instructions::create::process(program_id, accounts, deque_type, num_sectors),
        DequeInstruction::PushFront { value } => {
            instructions::push_front::process(program_id, accounts, value)
        }
        DequeInstruction::PushBack { value } => {
            instructions::push_back::process(program_id, accounts, value)
        }
        DequeInstruction::Remove { index } => {
            instructions::remove::process(program_id, accounts, index)
        }
        DequeInstruction::Resize { num_sectors } => {
            instructions::resize::process(program_id, accounts, num_sectors)
        }
    }
}
