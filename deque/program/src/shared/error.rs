use solana_program::program_error::ProgramError;

#[macro_export]
macro_rules! require {
  ($cond:expr, $err:expr)=> {
    if $cond {
        Ok(())
    } else {
        // Evaluate it once in case it's an expensive expression.
        let __err = ProgramError::from($err);
        #[cfg(target_os = "solana")]
        solana_program::msg!("[{}:{}] {}", std::file!(), std::line!(), __err);
        #[cfg(not(target_os = "solana"))]
        std::println!("[{}:{}] {}", std::file!(), std::line!(), __err);
        Err(__err)
    }
  };

  ($cond:expr, $err:expr, $($fmt_args:tt)+) => {
    if $cond {
        Ok(())
    } else {
        #[cfg(target_os = "solana")]
        solana_program::msg!("[{}:{}] {}", std::file!(), std::line!(), std::format_args!($($fmt_args)*));
        #[cfg(not(target_os = "solana"))]
        std::println!("[{}:{}] {}", std::file!(), std::line!(), std::format_args!($($fmt_args)*));
        Err($err)
    }
  };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DequeError {
    InvalidDiscriminant,
    InvalidInstructionTag,
    InvalidPackedData,
    InvalidMarketChoice,
    InvalidNumberOfAccounts,
    InsufficientVecCapacity,
    MalformedSlab,
    InvalidSectorIndex,
    ArithmetricError,
    AccountNotOwnedByProgram,
    AccountIsNotWritable,
    InvalidPDA,
    MustBeGreaterThanOne,
    NoActiveEscrow,
    OutOfBounds,
    DequeAccountUnallocated,
    EventAuthorityNotAllocated,
    EventAuthorityNotFullyAllocated,
    InvalidEventAuthorityBorrow,
    InsufficientAccountSpace,
    TransferError,
    RentGetError,
    ReallocError,
}

impl From<DequeError> for ProgramError {
    #[inline(always)]
    fn from(e: DequeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<DequeError> for &'static str {
    fn from(value: DequeError) -> Self {
        match value {
            DequeError::InvalidDiscriminant => "Invalid discriminant",
            DequeError::InvalidInstructionTag => "Invalid instruction tag",
            DequeError::InvalidPackedData => "Invalid packed data",
            DequeError::InvalidMarketChoice => "Invalid market choice",
            DequeError::InvalidNumberOfAccounts => "Invalid number of accounts passed",
            DequeError::InsufficientVecCapacity => "Insufficient vec.capacity()",
            DequeError::MalformedSlab => "Malformed slab data",
            DequeError::InvalidSectorIndex => "Invalid sector index",
            DequeError::ArithmetricError => "Checked arithmetic failed",
            DequeError::AccountIsNotWritable => "Account is not writable",
            DequeError::AccountNotOwnedByProgram => "Account is not owned by this program",
            DequeError::InvalidPDA => "Program derived address did not match input",
            DequeError::MustBeGreaterThanOne => "Argument is not >= 1",
            DequeError::NoActiveEscrow => "Trader has no active escrow",
            DequeError::OutOfBounds => "Index is out of bounds",
            DequeError::DequeAccountUnallocated => {
                "Deque account hasn't been allocated enough data"
            }
            DequeError::EventAuthorityNotAllocated => {
                "Event authority hasn't been allocated any data"
            }
            DequeError::EventAuthorityNotFullyAllocated => {
                "Event authority hasn't been allocated enough data"
            }
            DequeError::InvalidEventAuthorityBorrow => {
                "Couldn't borrow event authority account data"
            }
            DequeError::InsufficientAccountSpace => "Account doesn't have enough space",
            DequeError::TransferError => "Couldn't invoke system transfer",
            DequeError::RentGetError => "Failed to get rent",
            DequeError::ReallocError => "Failed to realloc",
        }
    }
}

#[cfg(not(target_os = "solana"))]
impl core::fmt::Display for DequeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(not(target_os = "solana"))]
impl std::error::Error for DequeError {}

pub type DequeProgramResult = Result<(), DequeError>;
