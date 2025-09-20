use solana_program::program_error::ProgramError;
use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instruction_enum::MarketEscrowChoice;
use crate::pack::vec_append_bytes;
use crate::require;
use crate::utils::sealed::Sealed;

pub mod event_authority;
pub(crate) mod event_emitter;

pub trait EmittableEvent: Sealed + Sized {
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
    pub market: &'p Pubkey,
    pub sender: &'p Pubkey,
    pub instruction: u8,
    pub nonce: u64,
    pub emitted_count: u16,
}

impl Sealed for EventHeader<'_> {}

impl EmittableEvent for EventHeader<'_> {
    const LEN: usize = 32 + 32 + 1 + 8 + 2;

    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>) {
        vec_append_bytes(buf, self.market.as_ref());
        vec_append_bytes(buf, self.sender.as_ref());
        vec_append_bytes(buf, &[self.instruction]);
        vec_append_bytes(buf, &self.nonce.to_le_bytes());
        vec_append_bytes(buf, &self.emitted_count.to_le_bytes());
    }

    #[cfg(feature = "client")]
    fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        todo!();
    }
}

pub struct DepositEvent<'p> {
    pub trader: &'p Pubkey,
    pub amount: u64,
    pub side: MarketEscrowChoice,
}

impl Sealed for DepositEvent<'_> {}

impl EmittableEvent for DepositEvent<'_> {
    const LEN: usize = 32 + 8;

    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>) {
        vec_append_bytes(buf, self.trader.as_ref());
        vec_append_bytes(buf, &self.amount.to_le_bytes());
        vec_append_bytes(buf, &[self.side.to_u8()]);
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
    pub side: MarketEscrowChoice,
}

impl Sealed for WithdrawEvent<'_> {}

impl EmittableEvent for WithdrawEvent<'_> {
    const LEN: usize = 32 + 8;

    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>) {
        vec_append_bytes(buf, self.trader.as_ref());
        vec_append_bytes(buf, &self.amount.to_le_bytes());
        vec_append_bytes(buf, &[self.side.to_u8()]);
    }

    #[cfg(feature = "client")]
    fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        todo!();
    }
}
