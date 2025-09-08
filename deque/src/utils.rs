use solana_program::{msg, program_error::ProgramError};

pub type Index = u32;
pub const NIL: Index = Index::MAX;

/// Below is taken directly from:
/// https://github.com/solana-program/libraries/blob/main/pod/src/primitives.rs
///
/// The standard `bool` is not a `Pod`, define a replacement that is
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub struct PodBool(pub u8);
impl PodBool {
    pub const fn from_bool(b: bool) -> Self {
        Self(if b { 1 } else { 0 })
    }
}

impl From<bool> for PodBool {
    fn from(b: bool) -> Self {
        Self::from_bool(b)
    }
}

/// Marker trait to narrow an account data's Pod-based outer Node<T> type (used as a slice of
/// mutable bytes: &mut [u8]) so that T can't also be passed.
pub trait SlabElement: bytemuck::Pod {}

pub fn from_slab_bytes<T: SlabElement>(data: &[u8], index: Index) -> Result<&T, ProgramError> {
    let i = index as usize;
    bytemuck::try_from_bytes(&data[i..i + std::mem::size_of::<T>()])
        .map_err(|_| ProgramError::InvalidAccountData)
}

pub fn from_slab_bytes_mut<T: SlabElement>(
    data: &mut [u8],
    index: Index,
) -> Result<&mut T, ProgramError> {
    let i = index as usize;
    bytemuck::try_from_bytes_mut(&mut data[i..i + std::mem::size_of::<T>()])
        .map_err(|_| ProgramError::InvalidAccountData)
}

pub fn log_bytes(bytes: &[u8]) {
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    msg!(&hex);
}
