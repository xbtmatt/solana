use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    syscalls::MAX_CPI_INSTRUCTION_DATA_LEN,
};

use crate::{
    context::event_emitter::EventEmitterContext,
    events::{event_authority, EmittableEvent, EventHeader},
    instruction_enum::DequeInstruction,
    PROGRAM_ID_PUBKEY,
};

const MAX_CPI_DATA_LEN: usize = MAX_CPI_INSTRUCTION_DATA_LEN as usize;
const EVENT_HEADER_SIZE: usize = 76;

pub struct EventEmitter<'a> {
    emit_instruction: Instruction,
    account_infos: [AccountInfo<'a>; 2],
}

impl<'info> EventEmitter<'info> {
    pub(crate) fn new<'a>(
        ctx: EventEmitterContext<'a, 'info>,
        market: &Pubkey,
        sender: &Pubkey,
        instruction_tag: u8,
    ) -> Result<Self, ProgramError> {
        // TODO: benchmark the cost of allocating the full max CPI data length up front as opposed
        // to resizing infrequently
        let mut data = Vec::with_capacity(MAX_CPI_INSTRUCTION_DATA_LEN as usize);

        // TODO: Separate the instruction tag from enum data, use only the tag here.
        // This will change several other things, too.
        DequeInstruction::FlushEventLog.pack_into_vec(&mut data);
        // TODO: Fill this with meaningful data.
        EventHeader {
            market,
            sender,
            instruction: instruction_tag,
            nonce: 1,
            emitted_count: 10,
        }
        .write(&mut data)?;

        Ok(Self {
            emit_instruction: Instruction {
                program_id: PROGRAM_ID_PUBKEY,
                accounts: vec![AccountMeta::new_readonly(
                    *ctx.event_authority.info.key,
                    true,
                )],
                data,
            },
            account_infos: [
                ctx.self_program.info.as_ref().clone(),
                ctx.event_authority.info.as_ref().clone(),
            ],
        })
    }

    pub fn flush(&mut self) -> ProgramResult {
        invoke_signed(
            &self.emit_instruction,
            &self.account_infos,
            &[event_authority::SEEDS],
        )?;
        self.emit_instruction.data.truncate(EVENT_HEADER_SIZE);
        Ok(())
    }

    pub fn add_event<Event: EmittableEvent>(&mut self, event: Event) -> ProgramResult {
        // TODO: Add an `index` field to all events to track order.

        if self.emit_instruction.data.len() + Event::LEN > MAX_CPI_DATA_LEN {
            self.flush()?;
        }

        event.write(&mut self.emit_instruction.data)?;
        Ok(())
    }
}
