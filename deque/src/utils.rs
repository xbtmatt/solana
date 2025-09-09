use solana_program::{msg, program_error::ProgramError};

pub type SlotIndex = u32;
pub const NIL: SlotIndex = SlotIndex::MAX;
pub const SECTOR_SIZE: usize = 100;

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
pub trait Slab: bytemuck::Pod {}

pub fn from_slab_bytes<T: Slab>(data: &[u8], index: SlotIndex) -> Result<&T, ProgramError> {
    let i = index as usize;
    bytemuck::try_from_bytes(&data[i..i + std::mem::size_of::<T>()])
        .map_err(|_| ProgramError::InvalidAccountData)
}

pub fn from_slab_bytes_mut<T: Slab>(
    data: &mut [u8],
    index: SlotIndex,
) -> Result<&mut T, ProgramError> {
    let i = index as usize;
    bytemuck::try_from_bytes_mut(&mut data[i..i + std::mem::size_of::<T>()])
        .map_err(|_| ProgramError::InvalidAccountData)
}

pub fn log_bytes(bytes: &[u8]) {
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    msg!(&hex);
}
