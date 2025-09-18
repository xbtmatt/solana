use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
    syscalls::MAX_CPI_INSTRUCTION_DATA_LEN,
};

use crate::{
    events::event_authority, instruction_enum::DequeInstruction,
    validation::self_program::SelfProgramInfo, PROGRAM_ID_PUBKEY,
};

const MAX_CPI_DATA_LEN: usize = MAX_CPI_INSTRUCTION_DATA_LEN as usize;
const MAX_EVENT_SIZE: usize = 96;

pub(crate) struct EventEmitter<'a> {
    emit_instruction: Instruction,
    account_infos: [AccountInfo<'a>; 2],
    scratch_buffer: [u8; MAX_EVENT_SIZE],
}

impl<'info> EventEmitter<'info> {
    pub fn new<'a>(
        deque_program: SelfProgramInfo<'a, 'info>,
        event_authority: SelfProgramInfo<'a, 'info>,
        deque_instruction: DequeInstruction,
    ) -> Result<Self, ProgramError> {
        let mut instruction_data = Vec::with_capacity(MAX_CPI_INSTRUCTION_DATA_LEN as usize);
        deque_instruction.pack_into_slice(&mut instruction_data);

        Ok(Self {
            emit_instruction: Instruction {
                program_id: PROGRAM_ID_PUBKEY,
                accounts: vec![AccountMeta::new_readonly(*event_authority.info.key, true)],
                data: instruction_data,
            },
            scratch_buffer: [0; MAX_EVENT_SIZE],
            account_infos: [
                deque_program.info.as_ref().clone(),
                event_authority.info.as_ref().clone(),
            ],
        })
    }

    pub fn flush(&mut self) -> ProgramResult {
        invoke_signed(
            &self.emit_instruction,
            &self.account_infos,
            &[event_authority::SEEDS, &[&[event_authority::BUMP]]],
        )?;
        Ok(())
    }

    pub fn add_to_buffer(&mut self, instruction: DequeInstruction) -> ProgramResult {
        // TODO: Add an `index` field to all events to track order.

        // Conservative check on the emit ixn data size; assume the event will be MAX_EVENT_SIZE.
        if self.emit_instruction.data.len() + instruction.get_size() > MAX_CPI_DATA_LEN {
            self.flush()?;
        }

        let start = self.emit_instruction.data.len();
        self.emit_instruction.data.resize(start + MAX_EVENT_SIZE, 0);
        // let written = instruction.pack_into_slice(&mut self.emit_instruction.data[start..])?;
        // self.emit_instruction.data.truncate(start + written);
        Ok(())
    }
}
