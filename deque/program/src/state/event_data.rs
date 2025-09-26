use bytemuck::{Pod, Zeroable};
use solana_program::{
    entrypoint::ProgramResult, program_error::ProgramError, syscalls::MAX_CPI_INSTRUCTION_DATA_LEN,
};
use static_assertions::const_assert_eq;

use crate::{
    shared::error::DequeError,
    syscalls::sol_memcpy_,
    utils::{from_slab_bytes_mut, write_bytes, Slab},
};

pub const EVENT_ACCOUNT_DISCRIMINANT: [u8; 8] = 0xbaadbaadf000000du64.to_le_bytes();

/// Equivalent to 10 CPI-based event log flushes.
/// This is an invariant because we never resize.
pub const EVENT_DATA_ACCOUNT_SIZE_INVARIANT: usize = (MAX_CPI_INSTRUCTION_DATA_LEN as usize) * 10;

const_assert_eq!(
    (EVENT_DATA_ACCOUNT_SIZE_INVARIANT < u32::MAX as usize),
    true
);

pub const EPHEMERAL_EVENT_LOG_HEADER_SIZE: usize = 16;

#[repr(C)]
#[derive(Pod, Zeroable, Copy, Clone)]
#[cfg_attr(not(target_os = "solana"), derive(Debug))]
pub struct EphemeralEventHeader {
    /// The account discriminant.
    discriminant: [u8; 8],
    /// Track the number of bytes to know where to append new events to.
    written_bytes_u32: [u8; 4],
    _padding: [u8; 4],
}

impl EphemeralEventHeader {
    pub fn init() -> Self {
        EphemeralEventHeader {
            discriminant: EVENT_ACCOUNT_DISCRIMINANT,
            written_bytes_u32: 0u32.to_le_bytes(),
            _padding: [0u8; 4],
        }
    }

    pub fn get_written_bytes(&self) -> u32 {
        u32::from_le_bytes(self.written_bytes_u32)
    }

    pub fn set_written_bytes(&mut self, value: u32) {
        self.written_bytes_u32 = value.to_le_bytes();
    }

    #[inline(always)]
    pub fn verify_discriminant(&self) -> ProgramResult {
        if self.discriminant != EVENT_ACCOUNT_DISCRIMINANT {
            return Err(DequeError::InvalidDiscriminant.into());
        }
        Ok(())
    }
}

impl Slab for EphemeralEventHeader {}

const_assert_eq!(core::mem::align_of::<EphemeralEventHeader>(), 1);
const_assert_eq!(core::mem::size_of::<EphemeralEventHeader>() % 8, 0);
const_assert_eq!(
    core::mem::size_of::<EphemeralEventHeader>(),
    EPHEMERAL_EVENT_LOG_HEADER_SIZE
);

#[repr(C)]
/// An ephemeral event log that records events that occurred in the span
/// of a single transaction, stored in the event authority's account data.
pub struct EphemeralEventLog<'a> {
    pub header: &'a mut EphemeralEventHeader,
    // `header.written_bytes_u32` worth of event data.
    pub event_data: &'a mut [u8],
}

impl<'a> EphemeralEventLog<'a> {
    /// Construct a new, empty EventData with allocated but uninitialized (zerod out) account data.
    pub fn init(zerod_account_data: &'a mut [u8]) -> ProgramResult {
        // Only needs to be at least the header size in order to init.
        if zerod_account_data.len() < EPHEMERAL_EVENT_LOG_HEADER_SIZE {
            return Err(DequeError::EventAuthorityNotAllocated.into());
        }

        let header = from_slab_bytes_mut(
            &mut zerod_account_data[0..EPHEMERAL_EVENT_LOG_HEADER_SIZE],
            0_usize,
        )?;
        *header = EphemeralEventHeader::init();

        Ok(())
    }

    pub fn reset_bytes_written_count(&mut self) {
        self.header.set_written_bytes(0);
    }

    /// Cast a byte vector to an event log and check the header's discriminant.
    pub fn from_bytes(data: &'a mut [u8]) -> Result<Self, ProgramError> {
        if data.len() != EVENT_DATA_ACCOUNT_SIZE_INVARIANT {
            return Err(DequeError::EventAuthorityNotFullyAllocated.into());
        }
        let (header_slab, event_data) = data.split_at_mut(EPHEMERAL_EVENT_LOG_HEADER_SIZE);
        let header = from_slab_bytes_mut::<EphemeralEventHeader>(header_slab, 0_usize)?;
        header.verify_discriminant()?;
        debug_assert!(header.get_written_bytes() as usize <= event_data.len());
        Ok(Self { header, event_data })
    }

    /// Cast a byte vector to an event log without checking the header's discriminant.
    pub fn from_bytes_unchecked(data: &'a mut [u8]) -> Result<Self, ProgramError> {
        if data.len() != EVENT_DATA_ACCOUNT_SIZE_INVARIANT {
            return Err(DequeError::EventAuthorityNotFullyAllocated.into());
        }
        let (header_slab, event_data) = data.split_at_mut(EPHEMERAL_EVENT_LOG_HEADER_SIZE);
        let header = from_slab_bytes_mut::<EphemeralEventHeader>(header_slab, 0_usize)?;
        debug_assert!(header.get_written_bytes() as usize <= event_data.len());
        Ok(Self { header, event_data })
    }

    /// Add data at the end of the current, well-formed account data buffer. Note that since this is
    /// rewritten to without repeatedly zero initializing the account data, it will often have
    /// garbage data at the end.
    /// # Safety
    /// Caller must guarantee that `new_event_data` and the data contained in the EphemeralEventLog
    /// are valid and non-overlapping.
    #[inline(always)]
    pub unsafe fn append_event_data(
        &mut self,
        new_event_data: *const u8,
        num_bytes: usize,
    ) -> ProgramResult {
        let written_bytes = self.header.get_written_bytes();
        let new_written_bytes = num_bytes
            .checked_add(written_bytes as usize)
            .ok_or(DequeError::InsufficientAccountSpace)?;
        let new_total_size = new_written_bytes
            .checked_add(EPHEMERAL_EVENT_LOG_HEADER_SIZE)
            .ok_or(DequeError::InsufficientAccountSpace)?;

        // This check also implicitly means it's not bigger than u32::MAX.
        if new_total_size > EVENT_DATA_ACCOUNT_SIZE_INVARIANT {
            return Err(DequeError::InsufficientAccountSpace.into());
        }

        sol_memcpy_(
            // SAFETY:
            // The pointer can't outlive the slice as long as it's not returned from this function.
            unsafe { self.event_data.as_mut_ptr().add(written_bytes as usize) },
            new_event_data,
            num_bytes as u64,
        );

        self.header.set_written_bytes(new_written_bytes as u32);

        debug_assert!(self.header.get_written_bytes() as usize <= self.event_data.len());

        Ok(())
    }
}
