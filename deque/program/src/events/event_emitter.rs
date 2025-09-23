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
    events::{EmittableEvent, HeaderEventData},
    instruction_enum::InstructionTag,
    seeds,
};

const MAX_CPI_DATA_LEN: usize = MAX_CPI_INSTRUCTION_DATA_LEN as usize;
const EVENT_HEADER_SIZE: usize = 76;

pub struct EventEmitter<'a> {
    pub emit_instruction: Instruction,
    pub account_infos: [AccountInfo<'a>; 2],
}

impl<'info> EventEmitter<'info> {
    pub(crate) fn new<'a>(
        ctx: EventEmitterContext<'a, 'info>,
        sender: &Pubkey,
        market: &Pubkey,
        instruction_tag: InstructionTag,
    ) -> Result<Self, ProgramError> {
        // TODO: benchmark the cost of allocating the full max CPI data length up front as opposed
        // to resizing infrequently
        let mut data: Vec<u8> = Vec::with_capacity(MAX_CPI_INSTRUCTION_DATA_LEN as usize);

        // Safety: `data` was just allocated with MAX_CPI_INSTRUCTION_DATA_LEN, and the length
        // is now exactly equal to one byte.
        unsafe {
            data.as_mut_ptr().write(InstructionTag::FlushEventLog as u8);
            data.set_len(1);
        }

        // TODO: Fill this with meaningful data.
        HeaderEventData::new(instruction_tag, market, sender, 1, 10).write(&mut data)?;

        Ok(Self {
            emit_instruction: Instruction {
                program_id: crate::ID,
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
            &[seeds::event_authority::SEEDS],
        )?;
        self.emit_instruction.data.truncate(EVENT_HEADER_SIZE);
        Ok(())
    }

    pub fn add_event<T: EmittableEvent>(&mut self, event: T) -> ProgramResult {
        // TODO: Add an `index` field to all events to track order.

        if self.emit_instruction.data.len() + T::LEN > MAX_CPI_DATA_LEN {
            self.flush()?;
        }

        event.write(&mut self.emit_instruction.data)?;
        Ok(())
    }
}
