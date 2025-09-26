use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    context::event_emitter::EventEmitterContext,
    events::event_emitter::EventEmitter,
    instruction_enum::{
        DepositInstructionData, InitializeDequeInstructionData, InstructionTag,
        ResizeInstructionData, WithdrawInstructionData,
    },
    instructions,
    pack::Pack,
    require,
    shared::error::DequeError,
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction_tag: InstructionTag = instruction_data[0].try_into()?;

    match instruction_tag {
        InstructionTag::FlushEventLog => instructions::flush::process(accounts)?,
        InstructionTag::InitializeEventAuthority => {
            instructions::initialize_event_authority::process(program_id, accounts)?
        }
        InstructionTag::ResizeEventAuthority => {
            instructions::resize_event_authority::process(program_id, accounts)?
        }
        _ => handle_instructions_with_events(
            program_id,
            accounts,
            instruction_data,
            instruction_tag,
        )?,
    }

    Ok(())
}

fn handle_instructions_with_events(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
    instruction_tag: InstructionTag,
) -> ProgramResult {
    debug_assert!(instruction_tag as u8 != InstructionTag::FlushEventLog as u8);

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
        InstructionTag::InitializeDeque => {
            let num_sectors = InitializeDequeInstructionData::unpack(instruction_data)?.num_sectors;
            instructions::initialize_deque::process(program_id, accounts, num_sectors)?;
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
