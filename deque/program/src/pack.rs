use std::mem::MaybeUninit;

use solana_program::{entrypoint::ProgramResult, program_error::ProgramError};

use crate::{
    require,
    shared::error::DequeError,
    utils::{write_bytes, UNINIT_BYTE},
};

pub trait Pack<const LEN: usize>: Sized {
    fn pack(&self) -> [u8; LEN] {
        let mut dst = [UNINIT_BYTE; LEN];
        self.pack_into_slice(&mut dst);
        // Safety: All LEN bytes were initialized in `pack_into_slice`.
        unsafe { *(dst.as_ptr() as *const [u8; LEN]) }
    }

    #[doc(hidden)]
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; LEN]);

    #[inline(always)]
    fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        Self::check_len(data)?;
        // Safety: The length was just checked.
        Ok(unsafe { Self::unpack_unchecked(data) })
    }

    #[inline(always)]
    fn check_len(data: &[u8]) -> ProgramResult {
        require!(data.len() >= LEN, DequeError::InvalidPackedData)
    }

    /// # Safety:
    /// The length of the instruction data must be verified before calling this.
    #[doc(hidden)]
    unsafe fn unpack_unchecked(instruction_data: &[u8]) -> Self;
}

pub trait Discriminant {
    const TAG: u8;
}

pub trait PackWithDiscriminant<const LEN: usize>: Pack<LEN> + Discriminant {
    #[inline(always)]
    fn check_tag(data: &[u8]) -> ProgramResult {
        // Safety: Length should be verified before calling this function, usually with `check_len`.
        require!(
            unsafe { *(data.get_unchecked(0)) } == Self::TAG,
            DequeError::InvalidDiscriminant
        )
    }

    #[inline(always)]
    fn unpack_checked_tag(data: &[u8]) -> Result<Self, ProgramError> {
        Self::check_len(data)?;
        Self::check_tag(data)?;
        // Safety: The length of the data and validity of the tag were just verified.
        Ok(unsafe { Self::unpack_unchecked(data) })
    }
}

// Blanket impl for Pack-able + Discriminant.
impl<T, const N: usize> PackWithDiscriminant<N> for T where T: Pack<N> + Discriminant {}

pub const U16_BYTES: usize = core::mem::size_of::<u16>();
pub const U64_BYTES: usize = core::mem::size_of::<u64>();

#[inline(always)]
/// # Safety
/// The caller must guarantee that `dst.capacity() >= new_len`.
/// If `dst.capacity() - dst.len()` is not >= `src.len()` the `write_bytes` iterator
/// will end early, resulting in a partially written `dst` with an incorrect  `len`.
pub unsafe fn vec_append_bytes(dst: &mut Vec<u8>, src: &[u8]) {
    write_bytes(dst.spare_capacity_mut(), src);

    // Safety:
    // 1. The elements at `[dst.len()..src.len()]` were just written to.
    // 2. Caller must guarantee that the `dst.capacity()` must be greater than the new length.
    unsafe {
        dst.set_len(dst.len() + src.len());
    }

    // TODO: Profile/compare the above call to the below..? I'm curious.
    // /// Append a slice of bytes to a heap-allocated Vec using the sol_memcpy syscall.
    // /// # Safety
    // // - dst.capacity() - dst.len() >= src.len()
    // // - `src` does not overlap `dst`
    // // - `dst` is writeable and `src` is readable
    // unsafe {
    //     // Get the pointer to the allocated but maybe uninitialized bytes at the end.
    //     let dst_ptr = dst.as_mut_ptr().add(dst.len());
    //     let src_ptr = src.as_ptr();
    //     // And write the bytes at `src` to it.
    //     sol_memcpy_(dst_ptr, src_ptr, src.len() as u64);
    //     // Publish the new bytes by manually setting the length.
    //     dst.set_len(dst.len() + src.len());
    // }
}

pub mod tests {
    #[test]
    pub fn raw_ptr_deref_unpack() {
        use crate::pack::U64_BYTES;

        let values = [1u64, 2u64, 3u64];
        let bytes: Vec<u8> = values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();

        assert_eq!(
            values[1],
            u64::from_le_bytes(unsafe {
                *(bytes.as_ptr().add(U64_BYTES) as *const [u8; U64_BYTES])
            }),
        );

        assert_eq!(
            values[1],
            u64::from_le_bytes(unsafe {
                *(bytes.get_unchecked(U64_BYTES..(U64_BYTES * 2)).as_ptr()
                    as *const [u8; U64_BYTES])
            }),
        )
    }
}
