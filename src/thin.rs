/*!
Rolling hash functions that support “thinning” by skipping many intermediate checksum values.
*/

#[allow(unused)]
use crate::util::*;
use crate::Hasher;

/// Extension trait describing rolling hash functions that can be efficiently implemented to
/// consume input data in larger *blocks*, instead of one byte at a time.
pub trait Thinned<Block: AsRef<[u8]>>: Hasher {
    /// The size in bytes of the blocks consumed by this implementation.
    const BLOCK_SIZE: usize;

    /// Computes this rolling hash function over a block of input data, returning only the final
    /// checksum and state.
    ///
    /// If this method is overriden by an implementor, the overriding definition must return the
    /// same values as the provided definition for identical inputs.
    fn process_block(
        &self,
        state: Self::State,
        old_block: &[u8],
        new_block: &Block,
    ) -> (Self::Checksum, Self::State) {
        assert_eq!(old_block.len(), Self::BLOCK_SIZE);

        self.process_sequence(
            state,
            old_block
                .iter()
                .copied()
                .zip(new_block.as_ref().iter().copied()),
        )
    }
}
