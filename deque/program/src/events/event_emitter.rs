use solana_program::{account_info::AccountInfo, syscalls::MAX_CPI_INSTRUCTION_DATA_LEN};

use crate::state::DequeInstruction;

const MAX_EVENT_SIZE: usize = 96;

pub(crate) struct EventEmitter<'info> {
    deque_prorgam: AccountInfo<'info>,
    event_emitter: AccountInfo<'info>,
    instruction: DequeInstruction,
    stack_buffer: [u8; MAX_EVENT_SIZE],
}

pub struct EmitterContext<'a, 'info> {
    deque_program: &'a AccountInfo<'info>,
    event_emitter: &'a AccountInfo<'info>,
}

impl<'info> EventEmitter<'info> {
    pub fn new<'a>(emitter_ctx: EmitterContext<'a, 'info>, instruction: DequeInstruction) {
        let EmitterContext {
            deque_program,
            event_emitter,
        } = emitter_ctx;
        let mut instruction_data = Vec::with_capacity(MAX_CPI_INSTRUCTION_DATA_LEN as usize);
    }
}
