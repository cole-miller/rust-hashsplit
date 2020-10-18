#[allow(unused)]
use crate::util::*;
use crate::Hasher;

pub trait Thinned<Block: AsRef<[u8]>>: Hasher {
    const BLOCK_SIZE: usize;

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
