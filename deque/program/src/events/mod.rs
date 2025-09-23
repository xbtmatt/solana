use solana_program::program_error::ProgramError;
use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instruction_enum::{InstructionTag, MarketChoice};
use crate::pack::{vec_append_bytes, Discriminant};
use crate::shared::error::DequeError;
use crate::{impl_discriminants, require};

pub(crate) mod event_emitter;

#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(strum_macros::FromRepr, strum_macros::EnumIter))]
pub enum EventTag {
    Header,
    Initialize,
    Deposit,
    Withdraw,
    Resize,
}

impl TryFrom<u8> for EventTag {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            // SAFETY: A valid enum variant is guaranteed with the match pattern.
            0..5 => Ok(unsafe { core::mem::transmute::<u8, Self>(value) }),
            _ => Err(DequeError::InvalidDiscriminant.into()),
        }
    }
}

#[cfg(not(target_os = "solana"))]
#[derive(Clone, Copy, Debug)]
pub enum DequeEvent<'p> {
    Header(HeaderEventData<'p>),
    // Initialize(InitializeEventData),
    Deposit(DepositEventData<'p>),
    Withdraw(WithdrawEventData<'p>),
    // Resize(ResizeEventData),
}

#[cfg(not(target_os = "solana"))]
impl<'p> DequeEvent<'p> {
    pub fn unpack(data: &'p [u8]) -> Result<DequeEvent<'p>, ProgramError> {
        let tag: EventTag = data[0].try_into()?;

        Ok(match tag {
            EventTag::Header => DequeEvent::Header(HeaderEventData::try_from_slice(data)?),
            EventTag::Deposit => DequeEvent::Deposit(DepositEventData::try_from_slice(data)?),
            EventTag::Withdraw => DequeEvent::Withdraw(WithdrawEventData::try_from_slice(data)?),
            _ => todo!(),
        })
    }
}

impl_discriminants!(
    HeaderEventData<'_>       => EventTag::Header,
    // InitializeEventData    => EventTag::Initialize,
    DepositEventData<'_>      => EventTag::Deposit,
    WithdrawEventData<'_>     => EventTag::Withdraw,
    // ResizeEventData        => EventTag::Resize,
);

pub trait EmittableEvent: Sized {
    const LEN: usize;

    /// Writes the event bytes to a destination buffer, checking that it has enough spare capacity.
    #[inline(always)]
    fn write(&self, buf: &mut Vec<u8>) -> ProgramResult {
        require!(
            buf.capacity() - buf.len() >= Self::LEN,
            DequeError::InsufficientVecCapacity,
            "Buffer spare capacity must be >= the length of the event"
        )?;
        // SAFETY: Spare capacity of the vec buffer was just checked.
        unsafe {
            self.write_unchecked(buf);
        }
        Ok(())
    }

    /// # Safety
    /// This function is an internal implementation that should only be called directly if the vec
    /// capacity has been deemed sufficient with regards to the packed struct size.
    #[doc(hidden)]
    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>);

    #[cfg(not(target_os = "solana"))]
    fn try_from_slice(data: &[u8]) -> Result<Self, ProgramError> {
        require!(
            data.len() >= Self::LEN,
            ProgramError::InvalidInstructionData
        )?;
        Ok(Self::from_slice_unchecked(data))
    }

    #[cfg(not(target_os = "solana"))]
    #[doc(hidden)]
    fn from_slice_unchecked(data: &[u8]) -> Self;

    #[inline(always)]
    fn check_len(data: &[u8]) -> ProgramResult {
        require!(data.len() >= Self::LEN, DequeError::InvalidPackedData)
    }
}

#[cfg_attr(not(target_os = "solana"), derive(Clone, Copy, Debug))]
pub struct HeaderEventData<'p> {
    pub discriminant: u8,
    pub instruction_tag: InstructionTag,
    pub market: &'p Pubkey,
    pub sender: &'p Pubkey,
    pub nonce: u64,
    pub emitted_count: u16,
}

impl EmittableEvent for HeaderEventData<'_> {
    const LEN: usize = 1 + 1 + 32 + 32 + 8 + 2;

    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>) {
        vec_append_bytes(buf, &[Self::TAG]);
        vec_append_bytes(buf, &[self.instruction_tag as u8]);
        vec_append_bytes(buf, self.market.as_ref());
        vec_append_bytes(buf, self.sender.as_ref());
        vec_append_bytes(buf, &self.nonce.to_le_bytes());
        vec_append_bytes(buf, &self.emitted_count.to_le_bytes());
    }

    #[cfg(not(target_os = "solana"))]
    fn from_slice_unchecked<'p>(data: &[u8]) -> Self {
        use arrayref::array_ref;

        Self {
            discriminant: data[0],
            instruction_tag: data[1]
                .try_into()
                .expect("Instruction tag should have already been validated."),
            // SAFETY: data[2..34] is exactly 32 bytes and Pubkey is repr(transparent) over [u8; 32]
            // Casted from input slice to preserve the 'p lifetime.
            market: unsafe { &*(data[2..34].as_ptr() as *const Pubkey) },
            // SAFETY: data[34..66] is exactly 32 bytes and Pubkey is repr(transparent) over [u8; 32]
            // Casted from input slice to preserve the 'p lifetime.
            sender: unsafe { &*(data[34..66].as_ptr() as *const Pubkey) },
            nonce: u64::from_le_bytes(*array_ref![data, 66, 8]),
            emitted_count: u16::from_le_bytes(*array_ref![data, 74, 2]),
        }
    }
}

impl<'p> HeaderEventData<'p> {
    fn new(
        instruction_tag: InstructionTag,
        market: &'p Pubkey,
        sender: &'p Pubkey,
        nonce: u64,
        emitted_count: u16,
    ) -> Self {
        HeaderEventData {
            discriminant: Self::TAG,
            market,
            instruction_tag,
            sender,
            nonce,
            emitted_count,
        }
    }
}

#[cfg_attr(not(target_os = "solana"), derive(Clone, Copy, Debug))]
pub struct DepositEventData<'p> {
    pub discriminant: u8,
    pub trader: &'p Pubkey,
    pub amount: u64,
    pub side: MarketChoice,
}

impl<'p> DepositEventData<'p> {
    pub fn new(trader: &'p Pubkey, amount: u64, side: MarketChoice) -> Self {
        Self {
            discriminant: Self::TAG,
            trader,
            amount,
            side,
        }
    }
}

impl EmittableEvent for DepositEventData<'_> {
    const LEN: usize = 1 + 32 + 8 + 1;

    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>) {
        vec_append_bytes(buf, &[Self::TAG]);
        vec_append_bytes(buf, self.trader.as_ref());
        vec_append_bytes(buf, &self.amount.to_le_bytes());
        vec_append_bytes(buf, &[self.side as u8]);
    }

    #[cfg(not(target_os = "solana"))]
    fn from_slice_unchecked(data: &[u8]) -> Self {
        use arrayref::array_ref;

        Self {
            discriminant: data[0],
            trader: unsafe { &*(data[1..33].as_ptr() as *const Pubkey) },
            amount: u64::from_le_bytes(*array_ref![data, 33, 8]),
            side: data[32]
                .try_into()
                .expect("Market choice enum should have been validated."),
        }
    }
}

#[repr(C)]
#[cfg_attr(not(target_os = "solana"), derive(Clone, Copy, Debug))]
pub struct WithdrawEventData<'p> {
    pub discriminant: u8,
    pub trader: &'p Pubkey,
    pub amount: u64,
    pub side: MarketChoice,
}

impl<'p> WithdrawEventData<'p> {
    pub fn new(trader: &'p Pubkey, amount: u64, side: MarketChoice) -> Self {
        Self {
            discriminant: Self::TAG,
            trader,
            amount,
            side,
        }
    }
}

impl EmittableEvent for WithdrawEventData<'_> {
    const LEN: usize = 1 + 32 + 8 + 1;

    unsafe fn write_unchecked(&self, buf: &mut Vec<u8>) {
        vec_append_bytes(buf, &[Self::TAG]);
        vec_append_bytes(buf, self.trader.as_ref());
        vec_append_bytes(buf, &self.amount.to_le_bytes());
        vec_append_bytes(buf, &[self.side as u8]);
    }

    #[cfg(not(target_os = "solana"))]
    fn from_slice_unchecked(data: &[u8]) -> Self {
        use arrayref::array_ref;

        Self {
            discriminant: data[0],
            trader: unsafe { &*(data[1..33].as_ptr() as *const Pubkey) },
            amount: u64::from_le_bytes(*array_ref![data, 33, 8]),
            side: data[32]
                .try_into()
                .expect("Market choice enum should have been validated."),
        }
    }
}
