use solana_program::program_error::ProgramError;

use crate::pack::{unpack_u16, unpack_u64};

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
    pub fn get_size(&self) -> usize {
        match self {
            DequeInstruction::Initialize { .. } => 3,
            DequeInstruction::Resize { .. } => 3,
            DequeInstruction::Deposit { .. } => 10,
            DequeInstruction::Withdraw { .. } => 2,
            DequeInstruction::FlushEventLog => 1,
        }
    }

    /// Extends a buffer with packed instruction bytes.
    pub fn pack_into_slice(&self, buf: &mut Vec<u8>) {
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
    }

    /// More ergonomic but over-allocates memory for ergonomics.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        self.pack_into_slice(&mut buf);
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
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}

pub mod tests {
    #[test]
    pub fn check_deque_sizes() {
        use super::*;

        let choice = MarketEscrowChoice::Base;
        let initialize = DequeInstruction::Initialize {
            num_sectors: u16::MAX,
        };
        let resize = DequeInstruction::Resize {
            num_sectors: u16::MAX,
        };
        let deposit = DequeInstruction::Deposit {
            choice: choice.clone(),
            amount: u64::MAX,
        };
        let withdraw = DequeInstruction::Withdraw { choice };
        let flush = DequeInstruction::FlushEventLog;
        assert_eq!(initialize.pack().len(), initialize.get_size());
        assert_eq!(resize.pack().len(), resize.get_size());
        assert_eq!(deposit.pack().len(), deposit.get_size());
        assert_eq!(withdraw.pack().len(), withdraw.get_size());
        assert_eq!(flush.pack().len(), flush.get_size());
    }
}
