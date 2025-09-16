#[macro_export]
macro_rules! require {
  ($cond:expr, $err:expr, $($fmt_args:tt)*) => {
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
