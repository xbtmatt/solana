use solana_program::account_info::next_account_info;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::context::event_emitter::EventEmitterContext;
use crate::events::event_emitter::EventEmitter;
use crate::instruction_enum::{
    DepositInstructionData, InitializeInstructionData, InstructionTag, ResizeInstructionData,
    WithdrawInstructionData,
};
use crate::pack::Pack;
use crate::shared::error::DequeError;
use crate::{instructions, require, seeds};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction_tag: InstructionTag = instruction_data[0].try_into()?;

    // If the instruction is for an event log flush, flush and return early, as the number of
    // account checks are only valid for non-flush instructions.
    if let InstructionTag::FlushEventLog = instruction_tag {
        let authority = next_account_info(&mut accounts.iter())?;
        require!(
            authority.is_signer,
            ProgramError::MissingRequiredSignature,
            "Event authority must be a signer"
        )?;
        require!(
            authority.key.as_ref() == seeds::event_authority::ID.as_ref(),
            ProgramError::IncorrectAuthority,
            "Invalid event authority"
        )?;
        msg!("Flushing! 🚽");
        return Ok(());
    }

    // Split [self program, event authority] from the rest of the accounts.
    require!(
        accounts.len() > 2,
        DequeError::InvalidNumberOfAccounts,
        "Expected at least {} accounts, got: {}",
        2,
        accounts.len()
    )?;
    let (event_emitter_accounts, accounts) = accounts.split_at(2);
    let event_ctx = EventEmitterContext::load(event_emitter_accounts)?;

    let mut event_emitter = EventEmitter::new(
        event_ctx,
        // TODO: Fix this logic here, these are *not* correct.
        accounts[0].key,
        accounts[1].key,
        instruction_tag,
    )?;

    match instruction_tag {
        InstructionTag::Initialize => {
            let num_sectors = InitializeInstructionData::unpack(instruction_data)?.num_sectors;
            instructions::initialize::process(program_id, accounts, num_sectors)?;
        }
        InstructionTag::Resize => {
            let num_sectors = ResizeInstructionData::unpack(instruction_data)?.num_sectors;
            instructions::resize::process(program_id, accounts, num_sectors)?;
        }
        InstructionTag::Deposit => {
            let deposit = DepositInstructionData::unpack(instruction_data)?;
            instructions::deposit::process(
                program_id,
                accounts,
                deposit.amount,
                deposit.choice,
                &mut event_emitter,
            )?;
            event_emitter.flush()?;
        }
        InstructionTag::Withdraw => {
            let withdraw = WithdrawInstructionData::unpack(instruction_data)?;
            instructions::withdraw::process(
                program_id,
                accounts,
                withdraw.choice,
                &mut event_emitter,
            )?;
            event_emitter.flush()?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
