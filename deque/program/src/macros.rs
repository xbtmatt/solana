#[macro_export]
macro_rules! market_seeds {
    ( $base_mint:expr, $quote_mint:expr ) => {
        &[
            $base_mint.as_ref(),
            $quote_mint.as_ref(),
            $crate::seeds::market::MARKET_SEED_STR,
        ]
    };
}

#[macro_export]
macro_rules! market_seeds_with_bump {
    ( $base_mint:expr, $quote_mint:expr, $bump:expr ) => {
        &[&[
            $base_mint.as_ref(),
            $quote_mint.as_ref(),
            $crate::seeds::market::MARKET_SEED_STR,
            &[$bump],
        ]]
    };
}

#[macro_export]
macro_rules! impl_discriminants {
    ( $( $ty:ty => $tag:path ),+ $(,)? ) => {
        $(
            impl $crate::pack::Discriminant for $ty {
                const TAG: u8 = $tag as u8;
            }
        )+
    };
}
