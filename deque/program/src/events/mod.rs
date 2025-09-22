use solana_program::program_error::ProgramError;
use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instruction_enum::{InstructionTag, MarketChoice};
use crate::pack::vec_append_bytes;
use crate::require;
use crate::utils::sealed::Sealed;

pub(crate) mod event_emitter;

pub trait EmittableEvent: Sealed + Sized {
    const DISCRIMINANT: u8;

    const LEN: usize;

    /// Writes the event bytes to a destination buffer, checking that it has enough spare capacity.
    #[inline(always)]
    fn write(&self, buf: &mut Vec<u8>) -> ProgramResult {
        require!(
            buf.capacity() - buf.len() >= Self::LEN,
            ProgramError::InvalidInstructionData,
            "Buffer spare capacity must be >= the length of the event"
        )?;
        // SAFETY: Spare capacity of the vec buffer is always checked prior.
        unsafe {
            self.write_unchecked(buf);
        }
        Ok(())
    }

    /// # Safety
    /// Check that the spare capacity of the buffer is large enough.
    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>);

    #[cfg(feature = "client")]
    fn unpack(data: &[u8]) -> Result<Self, ProgramError>;
}

pub struct EventHeader<'p> {
    pub discriminant: u8,
    pub instruction_tag: u8,
    pub market: &'p Pubkey,
    pub sender: &'p Pubkey,
    pub nonce: u64,
    pub emitted_count: u16,
}

impl Sealed for EventHeader<'_> {}

impl EmittableEvent for EventHeader<'_> {
    const DISCRIMINANT: u8 = 0;

    const LEN: usize = 1 + 1 + 32 + 32 + 8 + 2;

    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>) {
        vec_append_bytes(buf, &[Self::DISCRIMINANT]);
        vec_append_bytes(buf, &[self.instruction_tag]);
        vec_append_bytes(buf, self.market.as_ref());
        vec_append_bytes(buf, self.sender.as_ref());
        vec_append_bytes(buf, &self.nonce.to_le_bytes());
        vec_append_bytes(buf, &self.emitted_count.to_le_bytes());
    }

    #[cfg(feature = "client")]
    fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        todo!();
    }
}

impl<'p> EventHeader<'p> {
    fn new(
        instruction_tag: InstructionTag,
        market: &'p Pubkey,
        sender: &'p Pubkey,
        nonce: u64,
        emitted_count: u16,
    ) -> Self {
        EventHeader {
            market,
            discriminant: Self::DISCRIMINANT,
            instruction_tag: instruction_tag as u8,
            sender,
            nonce,
            emitted_count,
        }
    }
}

pub struct DepositEvent<'p> {
    pub trader: &'p Pubkey,
    pub amount: u64,
    pub side: MarketChoice,
}

impl Sealed for DepositEvent<'_> {}

impl EmittableEvent for DepositEvent<'_> {
    const DISCRIMINANT: u8 = 1;

    const LEN: usize = 1 + 1 + 32 + 8;

    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>) {
        vec_append_bytes(buf, &[Self::DISCRIMINANT]);
        vec_append_bytes(buf, self.trader.as_ref());
        vec_append_bytes(buf, &self.amount.to_le_bytes());
        vec_append_bytes(buf, &[self.side as u8]);
    }

    #[cfg(feature = "client")]
    fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        todo!();
    }
}

#[repr(C)]
pub struct WithdrawEvent<'p> {
    pub trader: &'p Pubkey,
    pub amount: u64,
    pub side: MarketChoice,
}

impl Sealed for WithdrawEvent<'_> {}

impl EmittableEvent for WithdrawEvent<'_> {
    const DISCRIMINANT: u8 = 2;

    const LEN: usize = 32 + 8;

    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>) {
        vec_append_bytes(buf, &[Self::DISCRIMINANT]);
        vec_append_bytes(buf, self.trader.as_ref());
        vec_append_bytes(buf, &self.amount.to_le_bytes());
        vec_append_bytes(buf, &[self.side as u8]);
    }

    #[cfg(feature = "client")]
    fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        todo!();
    }
}
