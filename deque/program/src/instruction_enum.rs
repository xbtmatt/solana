use core::mem::MaybeUninit;

use crate::pack::Discriminant;
use solana_program::program_error::ProgramError;

use crate::{
    impl_discriminants,
    pack::{Pack, PackWithDiscriminant, U16_BYTES},
    require,
    shared::error::DequeError,
    utils::write_bytes,
};

#[repr(u8)]
#[derive(Clone, Copy)]
#[cfg_attr(not(target_os = "solana"), derive(Default, Debug, Eq, PartialEq))]
pub enum MarketChoice {
    #[cfg_attr(not(target_os = "solana"), default)]
    Base,
    Quote,
}

impl TryFrom<u8> for MarketChoice {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            // SAFETY: A valid enum variant is guaranteed with the match pattern.
            0..=1 => Ok(unsafe { core::mem::transmute::<u8, MarketChoice>(value) }),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
#[cfg_attr(not(target_os = "solana"), derive(Debug, Eq, PartialEq))]
pub enum InstructionTag {
    Initialize,
    Resize,
    Deposit,
    Withdraw,
    FlushEventLog,
}

impl_discriminants! {
    InitializeInstructionData    => InstructionTag::Initialize,
    DepositInstructionData       => InstructionTag::Deposit,
    WithdrawInstructionData      => InstructionTag::Withdraw,
    ResizeInstructionData        => InstructionTag::Resize,
    FlushEventLogInstructionData => InstructionTag::FlushEventLog,
}

#[cfg(not(target_os = "solana"))]
pub enum DequeInstruction {
    Initialize(InitializeInstructionData),
    Deposit(DepositInstructionData),
    Withdraw(WithdrawInstructionData),
    Resize(ResizeInstructionData),
    FlushEventLog(FlushEventLogInstructionData),
}

#[cfg(not(target_os = "solana"))]
impl DequeInstruction {
    pub fn pack(&self) -> Vec<u8> {
        match self {
            DequeInstruction::Initialize(data) => data.pack().to_vec(),
            DequeInstruction::Deposit(data) => data.pack().to_vec(),
            DequeInstruction::Withdraw(data) => data.pack().to_vec(),
            DequeInstruction::Resize(data) => data.pack().to_vec(),
            DequeInstruction::FlushEventLog(data) => data.pack().to_vec(),
        }
    }
}

impl TryFrom<u8> for InstructionTag {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            // SAFETY: A valid enum variant is guaranteed with the match pattern.
            0..5 => Ok(unsafe { core::mem::transmute::<u8, Self>(value) }),
            _ => Err(DequeError::InvalidDiscriminant.into()),
        }
    }
}

#[repr(C)]
#[derive(Clone)]
#[cfg_attr(not(target_os = "solana"), derive(Debug, Eq, PartialEq))]
pub struct InitializeInstructionData {
    pub num_sectors: u16,
}

impl Pack<3> for InitializeInstructionData {
    #[inline(always)]
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; 3]) {
        dst[0].write(Self::TAG);
        write_bytes(&mut dst[1..3], &self.num_sectors.to_le_bytes());
    }

    #[inline(always)]
    unsafe fn unpack_unchecked(instruction_data: &[u8]) -> Self {
        // SAFETY: Caller guarantees instruction data has at least 2 bytes at offset 1.
        Self {
            num_sectors: u16::from_le_bytes(unsafe {
                *(instruction_data.get_unchecked(1..3).as_ptr() as *const [u8; U16_BYTES])
            }),
        }
    }
}

#[repr(C)]
#[derive(Clone)]
#[cfg_attr(not(target_os = "solana"), derive(Debug, Eq, PartialEq))]
pub struct ResizeInstructionData {
    pub num_sectors: u16,
}

impl Pack<3> for ResizeInstructionData {
    #[inline(always)]
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; 3]) {
        dst[0].write(Self::TAG);
        write_bytes(&mut dst[1..3], &self.num_sectors.to_le_bytes());
    }

    #[inline(always)]
    unsafe fn unpack_unchecked(instruction_data: &[u8]) -> Self {
        // SAFETY: Caller guarantees instruction data has at least 2 bytes at offset 1.
        let num_sectors = u16::from_le_bytes(unsafe {
            *(instruction_data.get_unchecked(1..3).as_ptr() as *const [u8; U16_BYTES])
        });
        Self { num_sectors }
    }
}

#[repr(C)]
#[derive(Clone)]
#[cfg_attr(not(target_os = "solana"), derive(Debug, Eq, PartialEq))]
pub struct DepositInstructionData {
    pub choice: MarketChoice,
    pub amount: u64,
}

#[cfg(not(target_os = "solana"))]
impl DepositInstructionData {
    pub fn new(amount: u64, choice: MarketChoice) -> Self {
        DepositInstructionData { amount, choice }
    }
}

#[cfg(not(target_os = "solana"))]
impl WithdrawInstructionData {
    pub fn new(choice: MarketChoice) -> Self {
        WithdrawInstructionData { choice }
    }
}

// TODO: Check the actual sBPF output and see if the extensible impl we have here compiles to
// roughly the same bytecode as inlining it with no extensibility/composition on the checks and
// whatnot.
impl Pack<10> for DepositInstructionData {
    #[inline(always)]
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; 10]) {
        dst[0].write(Self::TAG);
        dst[1].write(self.choice as u8);
        write_bytes(&mut dst[2..10], &self.amount.to_le_bytes());
    }

    #[inline(always)]
    fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        Self::check_len(data)?;
        Self::check_tag(data)?;
        require!(
            MarketChoice::try_from(unsafe { *(data.get_unchecked(1)) }).is_ok(),
            DequeError::InvalidMarketChoice
        )?;
        // Safety: The length, tag, and choice enum were all just verified.
        Ok(unsafe { Self::unpack_unchecked(data) })
    }

    #[inline(always)]
    unsafe fn unpack_unchecked(instruction_data: &[u8]) -> Self {
        // SAFETY: Caller guarantees instruction data has 1 byte at offset 1.
        let choice_byte = unsafe { *(instruction_data.get_unchecked(1)) };
        // SAFETY: Caller must ensure that that byte is either 0 or 1.
        let choice = unsafe { core::mem::transmute::<u8, MarketChoice>(choice_byte) };
        // SAFETY: Caller guarantees instruction data has 8 bytes at offset 2.
        let amount = u64::from_le_bytes(unsafe {
            *(instruction_data.get_unchecked(2..10).as_ptr() as *const [u8; 8])
        });
        Self { choice, amount }
    }
}

#[repr(C)]
#[derive(Clone)]
#[cfg_attr(not(target_os = "solana"), derive(Debug, Eq, PartialEq))]
pub struct WithdrawInstructionData {
    pub choice: MarketChoice,
}

impl Pack<2> for WithdrawInstructionData {
    #[inline(always)]
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; 2]) {
        dst[0].write(Self::TAG);
        dst[1].write(self.choice as u8);
    }

    #[inline(always)]
    unsafe fn unpack_unchecked(instruction_data: &[u8]) -> Self {
        // SAFETY: Caller guarantees instruction data has 1 byte at offset 1.
        let choice_byte = unsafe { *(instruction_data.get_unchecked(1)) };
        // SAFETY: Caller must ensure that that byte is either 0 or 1.
        let choice = unsafe { core::mem::transmute::<u8, MarketChoice>(choice_byte) };
        Self { choice }
    }
}

#[repr(C)]
#[derive(Clone)]
#[cfg_attr(not(target_os = "solana"), derive(Debug, Eq, PartialEq))]
pub struct FlushEventLogInstructionData {}

impl Pack<1> for FlushEventLogInstructionData {
    #[inline(always)]
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; 1]) {
        dst[0].write(Self::TAG);
    }

    #[inline(always)]
    unsafe fn unpack_unchecked(_instruction_data: &[u8]) -> Self {
        Self {}
    }
}

pub mod tests {
    #[test]
    pub fn u8_to_market_choice() {
        use super::MarketChoice;
        let data = [0, 1, 2, 3, 4];
        assert!(MarketChoice::try_from(unsafe { *(data.get_unchecked(0)) }).is_ok());
        assert!(MarketChoice::try_from(unsafe { *(data.get_unchecked(1)) }).is_ok());
        assert!(MarketChoice::try_from(unsafe { *(data.get_unchecked(2)) }).is_err());
        assert!(MarketChoice::try_from(unsafe { *(data.get_unchecked(3)) }).is_err());
        assert!(MarketChoice::try_from(unsafe { *(data.get_unchecked(4)) }).is_err());
        assert!(MarketChoice::try_from(unsafe { *(data.as_ptr().add(0)) }).is_ok());
        assert!(MarketChoice::try_from(unsafe { *(data.as_ptr().add(1)) }).is_ok());
        assert!(MarketChoice::try_from(unsafe { *(data.as_ptr().add(2)) }).is_err());
        assert!(MarketChoice::try_from(unsafe { *(data.as_ptr().add(3)) }).is_err());
        assert!(MarketChoice::try_from(unsafe { *(data.as_ptr().add(4)) }).is_err());
    }
}
