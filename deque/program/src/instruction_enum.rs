use solana_program::program_error::ProgramError;

use crate::pack::{unpack_u16, unpack_u64};

#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(Default))]
pub enum MarketEscrowChoice {
    #[cfg_attr(test, default)]
    Base,
    Quote,
}

impl MarketEscrowChoice {
    pub fn to_u8(&self) -> u8 {
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
#[cfg_attr(test, derive(strum_macros::FromRepr, strum_macros::EnumIter))]
pub enum DequeInstruction {
    Initialize {
        num_sectors: u16,
    } = 0,
    Resize {
        num_sectors: u16,
    } = 1,
    Deposit {
        choice: MarketEscrowChoice,
        amount: u64,
    } = 2,
    Withdraw {
        choice: MarketEscrowChoice,
    } = 3,
    FlushEventLog = 4,
}

impl DequeInstruction {
    pub const fn get_size(&self) -> usize {
        match self {
            DequeInstruction::Initialize { .. } => 3,
            DequeInstruction::Resize { .. } => 3,
            DequeInstruction::Deposit { .. } => 10,
            DequeInstruction::Withdraw { .. } => 2,
            DequeInstruction::FlushEventLog => 1,
        }
    }

    /// Extends a buffer with packed instruction bytes.
    pub fn pack_into_vec(&self, buf: &mut Vec<u8>) {
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
                buf.push(2);
                buf.push(choice.to_u8());
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::Withdraw { ref choice } => {
                buf.push(3);
                buf.push(choice.to_u8());
            }
            Self::FlushEventLog => {
                buf.push(4);
            }
        }
    }

    /// More ergonomic but over-allocates memory.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.get_size());
        self.pack_into_vec(&mut buf);
        buf
    }

    #[inline(always)]
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
            4 => DequeInstruction::FlushEventLog,
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}

pub mod tests {
    #[test]
    pub fn check_deque_sizes() {
        use super::DequeInstruction;
        use strum::IntoEnumIterator;

        for ixn in DequeInstruction::iter() {
            assert_eq!(ixn.pack().len(), ixn.get_size());
        }
    }
}
