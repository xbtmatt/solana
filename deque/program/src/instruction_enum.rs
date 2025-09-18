use solana_program::program_error::ProgramError;

#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum MarketEscrowChoice {
    Base,
    Quote,
}

impl TryFrom<u8> for MarketEscrowChoice {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            // SAFETY: A valid enum variant is guaranteed with the match pattern.
            0..=1 => Ok(unsafe { core::mem::transmute::<u8, MarketEscrowChoice>(value) }),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum DequeInstruction {
    Initialize {
        num_sectors: u16,
    },
    Resize {
        num_sectors: u16,
    },
    Deposit {
        amount: u64,
        choice: MarketEscrowChoice,
    },
    Withdraw {
        choice: MarketEscrowChoice,
    },
    FlushEventLog,
}

#[repr(C)]
#[derive(Clone, Debug, Pod)]
pub struct InitializeInstructionData = {}


impl TryFrom<u8> for DequeInstruction {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            // SAFETY: A valid enum variant is guaranteed with the match pattern.
            0..=4 => Ok(unsafe { core::mem::transmute::<u8, DequeInstruction>(value) }),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

impl DequeInstruction {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, instruction_data) = data
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match tag {
            0 => {
                let num_sectors = unpack_u64(instruction_data);
                DequeInstruction::Initialize { num_sectors }
            }
        })
    }
}

const U16_BYTES: usize = core::mem::size_of::<u16>();

#[inline(always)]
pub fn unpack_u16(instruction_data: &[u8]) -> Result<u16, ProgramError> {
    if instruction_data.len() >= U64_BYTES {
        // SAFETY: `instruction_data` is at least `U16_BYTES`.
        Ok(unsafe { u16::from_le_bytes(*(instruction_data.as_ptr() as *const [u8; U16_BYTES])) })
    } else {
        Err(ProgramError::InvalidInstructionData)
    }
}

const U64_BYTES: usize = core::mem::size_of::<u64>();

#[inline(always)]
pub fn unpack_u64(instruction_data: &[u8]) -> Result<u64, ProgramError> {
    if instruction_data.len() >= U64_BYTES {
        // SAFETY: `instruction_data` is at least `U64_BYTES`.
        Ok(unsafe { u64::from_le_bytes(*(instruction_data.as_ptr() as *const [u8; U64_BYTES])) })
    } else {
        Err(ProgramError::InvalidInstructionData)
    }
}
