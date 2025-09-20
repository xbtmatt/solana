use solana_program::account_info::next_account_info;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::context::event_emitter::EventEmitterContext;
use crate::events::event_authority;
use crate::events::event_emitter::EventEmitter;
use crate::instruction_enum::DequeInstruction;
use crate::utils::log_bytes;
use crate::{instructions, require};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("__INSTRUCTION DATA__");
    log_bytes(instruction_data);

    let instruction = DequeInstruction::unpack(instruction_data)?;

    if let DequeInstruction::FlushEventLog = instruction {
        let authority = next_account_info(&mut accounts.iter())?;
        require!(
            authority.is_signer,
            ProgramError::MissingRequiredSignature,
            "Event authority must be a signer"
        )?;
        require!(
            authority.key.as_ref() == event_authority::ID.as_ref(),
            ProgramError::IncorrectAuthority,
            "Invalid event authority"
        )?;
        return Ok(());
    };

    // Split [self program, event authority] from the rest of the accounts.
    let (event_emitter_accounts, accounts) = accounts.split_at(2);
    let event_ctx = EventEmitterContext::load(event_emitter_accounts)?;

    let mut event_emitter = EventEmitter::new(
        event_ctx,
        // TODO: Fix this logic here, these are *not* correct.
        accounts[0].key,
        accounts[1].key,
        instruction_data[0],
    )?;

    match instruction {
        DequeInstruction::Initialize { num_sectors } => {
            instructions::initialize::process(program_id, accounts, num_sectors)?
        }
        DequeInstruction::Resize { num_sectors } => {
            instructions::resize::process(program_id, accounts, num_sectors)?
        }
        DequeInstruction::Deposit { amount, choice } => {
            instructions::deposit::process(
                program_id,
                accounts,
                amount,
                choice,
                &mut event_emitter,
            )?;
            event_emitter.flush()?;
        }
        DequeInstruction::Withdraw { choice } => {
            instructions::withdraw::process(program_id, accounts, choice, &mut event_emitter)?;
            event_emitter.flush()?;
        }
        DequeInstruction::FlushEventLog => msg!("Flushing! 🚽"),
    }

    Ok(())
}
