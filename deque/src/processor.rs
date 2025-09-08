use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

use crate::state::DequeInstruction;
use crate::{instructions, log_bytes};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("logging process instruction");
    log_bytes(instruction_data);

    let instruction = DequeInstruction::try_from_slice(instruction_data)?;

    match instruction {
        DequeInstruction::Initialize { deque_type } => {
            instructions::create::process(program_id, accounts, deque_type)
        }
        DequeInstruction::PushFront { value } => {
            instructions::push_front::process(program_id, accounts, value)
        }
        DequeInstruction::PushBack { value } => {
            instructions::push_back::process(program_id, accounts, value)
        }
        DequeInstruction::Remove { index } => {
            instructions::remove::process(program_id, accounts, index)
        }
    }
}
