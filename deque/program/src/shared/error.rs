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
    InvalidPackedData,
    InvalidMarketChoice,
    InvalidNumberOfAccounts,
}

impl From<DequeError> for ProgramError {
    fn from(e: DequeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<DequeError> for &'static str {
    fn from(value: DequeError) -> Self {
        match value {
            DequeError::InvalidDiscriminant => "Invalid discriminant",
            DequeError::InvalidPackedData => "Invalid packed data",
            DequeError::InvalidMarketChoice => "Invalid market choice",
            DequeError::InvalidNumberOfAccounts => "Invalid number of accounts passed",
        }
    }
}

#[cfg(not(target_os = "solana"))]
impl core::fmt::Display for DequeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}
