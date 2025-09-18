use solana_program::program_error::ProgramError;

#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum MarketEscrowChoice {
    Base,
    Quote,
}

impl MarketEscrowChoice {
    fn into(&self) -> u8 {
        match self {
            MarketEscrowChoice::Base => 0,
            MarketEscrowChoice::Quote => 1,
        }
    }
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
        choice: MarketEscrowChoice,
        amount: u64,
    },
    Withdraw {
        choice: MarketEscrowChoice,
    },
    FlushEventLog,
}

impl DequeInstruction {
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            Self::Initialize { num_sectors } => {
                buf.push(0);
                buf.extend_from_slice(&num_sectors.to_le_bytes());
            }
            Self::Resize { num_sectors } => {
                buf.push(1);
                buf.extend_from_slice(&num_sectors.to_le_bytes());
            }
            Self::Deposit { ref choice, amount } => {
                buf.push(1);
                buf.push(choice.into());
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::Withdraw { ref choice } => {
                buf.push(3);
                buf.push(choice.into());
            }
            Self::FlushEventLog => {
                buf.push(4);
            }
        }

        buf
    }

    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, instruction_data) = data
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match tag {
            0 => {
                let num_sectors = unpack_u16(instruction_data)?;
                DequeInstruction::Initialize { num_sectors }
            }
            1 => {
                let num_sectors = unpack_u16(instruction_data)?;
                DequeInstruction::Resize { num_sectors }
            }

            2 => {
                let (choice_bytes, amount_bytes) = instruction_data
                    .split_first()
                    .ok_or(ProgramError::InvalidInstructionData)?;
                DequeInstruction::Deposit {
                    choice: MarketEscrowChoice::try_from(*choice_bytes)?,
                    amount: unpack_u64(amount_bytes)?,
                }
            }
            3 => DequeInstruction::Withdraw {
                choice: MarketEscrowChoice::try_from(instruction_data[0])?,
            },
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}

const U16_BYTES: usize = core::mem::size_of::<u16>();

#[inline(always)]
pub fn unpack_u16(instruction_data: &[u8]) -> Result<u16, ProgramError> {
    if instruction_data.len() >= U16_BYTES {
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
