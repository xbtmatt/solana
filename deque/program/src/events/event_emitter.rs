use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
    syscalls::MAX_CPI_INSTRUCTION_DATA_LEN,
};

use crate::{instruction_enum::DequeInstruction, PROGRAM_ID_PUBKEY};

const MAX_CPI_DATA_LEN: usize = MAX_CPI_INSTRUCTION_DATA_LEN as usize;
const MAX_EVENT_SIZE: usize = 96;

pub(crate) struct EventEmitter<'info> {
    deque_program: AccountInfo<'info>,
    event_emitter: AccountInfo<'info>,
    emit_instruction: Instruction,
    scratch_buffer: [u8; MAX_EVENT_SIZE],
}

pub struct EmitterContext<'a, 'info> {
    deque_program: &'a AccountInfo<'info>,
    event_emitter: &'a AccountInfo<'info>,
}

impl<'info> EventEmitter<'info> {
    pub fn new<'a>(
        emitter_ctx: EmitterContext<'a, 'info>,
        deque_instruction: DequeInstruction,
    ) -> Result<Self, ProgramError> {
        let EmitterContext {
            deque_program,
            event_emitter,
        } = emitter_ctx;
        let mut instruction_data = Vec::with_capacity(MAX_CPI_INSTRUCTION_DATA_LEN as usize);
        deque_instruction.pack_into(&mut instruction_data);

        Ok(Self {
            deque_program: deque_program.clone(),
            event_emitter: event_emitter.clone(),
            emit_instruction: Instruction {
                program_id: PROGRAM_ID_PUBKEY,
                accounts: vec![AccountMeta::new_readonly(*event_emitter.key, true)],
                data: instruction_data,
            },
            scratch_buffer: [0; MAX_EVENT_SIZE],
        })
    }

    pub fn flush(&mut self) -> ProgramResult {
        invoke_signed(
            &self.emit_instruction,
            &[
                self.deque_program.as_ref().clone(),
                self.event_emitter.as_ref().clone(),
            ],
            todo!(),
            // Add signer seeds macro here.
            // TODO: Add a `pub mod { bump() } to avoid having to recalculate the bump in these seed generation macros`
        );
        Ok(())
    }

    pub fn add_to_buffer(&mut self, instruction: DequeInstruction) -> ProgramResult {
        // TODO: Add an `index` field to all events to track order.

        // Conservative check on the emit ixn data size; assume the event will be MAX_EVENT_SIZE.
        if self.emit_instruction.data.len() + MAX_EVENT_SIZE > MAX_CPI_DATA_LEN {
            self.flush()?;
        }

        let start = self.emit_instruction.data.len();
        self.emit_instruction.data.resize(start + MAX_EVENT_SIZE, 0);
        let written = instruction.pack_into_slice(&mut self.emit_instruction.data[start..])?;
        self.emit_instruction.data.truncate(start + written);
        Ok(())
    }
}
