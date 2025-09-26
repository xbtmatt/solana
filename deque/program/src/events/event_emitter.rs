use solana_program::{
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
    shared::error::DequeError,
    state::{EphemeralEventLog, EPHEMERAL_EVENT_LOG_HEADER_SIZE},
    validation::{event_authority::EventAuthorityInfo, self_program::SelfProgramInfo},
};

const MAX_CPI_DATA_LEN: usize = MAX_CPI_INSTRUCTION_DATA_LEN as usize;
/// The event header size with the instruction tag prepended.
const FULL_HEADER_SIZE: usize = HeaderEventData::LEN + size_of::<InstructionTag>();

pub struct EventEmitter<'a, 'info> {
    pub emit_instruction: Instruction,
    pub self_program: SelfProgramInfo<'a, 'info>,
    pub event_authority: EventAuthorityInfo<'a, 'info>,
}

impl<'a, 'info> EventEmitter<'a, 'info> {
    pub(crate) fn new(
        ctx: EventEmitterContext<'a, 'info>,
        sender: &Pubkey,
        market: &Pubkey,
        triggering_instruction_tag: InstructionTag,
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
        HeaderEventData::new(triggering_instruction_tag, market, sender, 1, 10).write(&mut data)?;

        // Reset the event authority's account data if it's going to be written to.
        let should_reset =
            ctx.event_authority.info.is_writable && !ctx.event_authority.info.data_is_empty();

        if should_reset {
            if let Ok(ref mut event_authority_data) =
                &mut ctx.event_authority.info.try_borrow_mut_data()
            {
                EphemeralEventLog::from_bytes(event_authority_data)?.reset_bytes_written_count();
            }
        }

        Ok(Self {
            emit_instruction: Instruction {
                program_id: crate::ID,
                accounts: vec![AccountMeta::new_readonly(
                    *ctx.event_authority.info.key,
                    true,
                )],
                data,
            },
            self_program: ctx.self_program.clone(),
            event_authority: ctx.event_authority.clone(),
        })
    }

    pub fn flush(&mut self) -> ProgramResult {
        // Cast the event authority's account data to a mutable ephemeral event log.
        let mut event_authority_data = self
            .event_authority
            .info
            .data
            .try_borrow_mut()
            .or(Err(DequeError::InvalidEventAuthorityBorrow))?;

        let mut ephemeral_event_log =
            EphemeralEventLog::from_bytes_unchecked(&mut event_authority_data)?;

        solana_program::msg!("writing {} bytes to the ephemeral event log");
        {
            solana_program::msg!(
                "bytes bein---------------------------------------------------g: {:?}",
                unsafe {
                    core::slice::from_raw_parts(
                        self.emit_instruction.data.as_ptr().add(FULL_HEADER_SIZE),
                        self.emit_instruction.data.len() - FULL_HEADER_SIZE,
                    )
                }
            );
        }

        // TODO: As it stands, the data in the header is *not* actually duplicated.
        // This would have to be refactored to make sure that the CPI header data is always synced
        // with the ephemeral event log header data.
        // Write the event-specific data in the instruction buffer to the ephemeral event log.
        unsafe {
            ephemeral_event_log.append_event_data(
                self.emit_instruction.data.as_ptr().add(FULL_HEADER_SIZE),
                self.emit_instruction.data.len() - FULL_HEADER_SIZE,
            )?;
        }

        drop(event_authority_data);

        // Invoke the cpi to emit instruction data events.
        invoke_signed(
            &self.emit_instruction,
            &[
                self.self_program.info.clone(),
                self.event_authority.info.clone(),
            ],
            &[seeds::event_authority::SEEDS],
        )?;

        self.emit_instruction.data.truncate(FULL_HEADER_SIZE);
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
