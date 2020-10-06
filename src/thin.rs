/*!
Rolling hash functions that support "thinning" by skipping many intermediate checksum values.
*/

use crate::Hasher;

use core::num::NonZeroUsize;

/// Extension trait describing rolling hash functions that can be efficiently implemented to
/// consume input data in larger *blocks*, instead of one byte at a time.
pub trait Thinned<Block>: Hasher
where
    Block: AsRef<[u8]>,
{
    /// The size in bytes of the blocks consumed by this implementation.
    const BLOCK_SIZE: NonZeroUsize;

    /// Computes this rolling hash function over a block of input data, returning only the final
    /// checksum and state.
    ///
    /// If this method is overriden by an implementor, the overriding definition must return the
    /// same values as the provided definition for identical inputs.
    fn process_block(
        &self,
        state: Self::State,
        width: NonZeroUsize,
        old_data: &[u8],
        new_data: &Block,
    ) -> (Self::Checksum, Self::State) {
        self.process_slice(state, width, old_data, new_data.as_ref())
    }
}
