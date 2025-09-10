use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError};

use crate::PROGRAM_ID_PUBKEY;

/// The ordinal `sector` index in the slab of bytes dedicated to inner data for a type.
/// That is, to get the raw bytes offset, it is multiplied by the sector type's sector size.
pub type SectorIndex = u32;
pub const NIL: SectorIndex = SectorIndex::MAX;

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

#[inline(always)]
pub fn from_slab_bytes<T: Slab>(data: &[u8], byte_offset: usize) -> Result<&T, ProgramError> {
    let i = byte_offset;
    bytemuck::try_from_bytes(&data[i..i + std::mem::size_of::<T>()])
        .map_err(|_| ProgramError::InvalidAccountData)
}

#[inline(always)]
pub fn from_slab_bytes_mut<T: Slab>(
    data: &mut [u8],
    byte_offset: usize,
) -> Result<&mut T, ProgramError> {
    let i = byte_offset;
    bytemuck::try_from_bytes_mut(&mut data[i..i + std::mem::size_of::<T>()])
        .map_err(|_| ProgramError::InvalidAccountData)
}

#[inline(always)]
pub fn from_sector_idx<T: Slab>(sectors: &[u8], idx: SectorIndex) -> Result<&T, ProgramError> {
    if idx == NIL {
        return Err(ProgramError::InvalidAccountData);
    }
    let stride = size_of::<T>();
    let start = (idx as usize)
        .checked_mul(stride)
        .ok_or(ProgramError::InvalidAccountData)?;
    from_slab_bytes(sectors, start)
}

#[inline(always)]
pub fn from_sector_idx_mut<T: Slab>(
    sectors: &mut [u8],
    idx: SectorIndex,
) -> Result<&mut T, ProgramError> {
    if idx == NIL {
        return Err(ProgramError::InvalidAccountData);
    }
    let stride = size_of::<T>();
    let start = (idx as usize)
        .checked_mul(stride)
        .ok_or(ProgramError::InvalidAccountData)?;
    from_slab_bytes_mut(sectors, start)
}

pub fn log_bytes(bytes: &[u8]) {
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    msg!(&hex);
}

#[inline(always)]
pub fn check_owned_and_writable(account: &AccountInfo) -> Result<(), ProgramError> {
    if account.owner.as_array() != PROGRAM_ID_PUBKEY.as_array() {
        msg!("account not owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    if !account.is_writable {
        msg!("account not writable");
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}
