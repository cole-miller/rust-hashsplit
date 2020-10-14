use crate::Hasher;

use alloc::{borrow::Cow, boxed::Box};
use core::ops::Deref;

pub struct ResumableChunk<'a, Hash: Hasher> {
    chunk: Cow<'a, [u8]>,
    pub state: Hash::State,
}

impl<'a, Hash> ResumableChunk<'a, Hash>
where
    Hash: Hasher,
{
    pub fn new<X: Into<Cow<'a, [u8]>>>(data: X, state: Hash::State) -> Self {
        Self {
            chunk: data.into(),
            state,
        }
    }
}

impl<'a, Hash> Deref for ResumableChunk<'a, Hash>
where
    Hash: Hasher,
{
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.chunk
    }
}

pub enum TreeNode<'a, Hash: Hasher> {
    Internal(Box<[Self]>),
    Leaf(Box<[ResumableChunk<'a, Hash>]>),
}

pub struct Tree<'a, Hash: Hasher> {
    pub root: Box<TreeNode<'a, Hash>>,
}
