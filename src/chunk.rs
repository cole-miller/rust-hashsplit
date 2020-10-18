#[allow(unused)]
use crate::util::*;
use crate::Hasher;

use alloc::{borrow::Cow, boxed::Box};
use core::ops::Deref;

pub struct ResumableChunk<'a, Hash: Hasher> {
    chunk: Cow<'a, [u8]>,
    pub state: Hash::State,
}

impl<'a, Hash: Hasher> ResumableChunk<'a, Hash> {
    pub fn new<T: Into<Cow<'a, [u8]>>>(data: T, state: Hash::State) -> Self {
        Self {
            chunk: data.into(),
            state,
        }
    }
}

impl<'a, Hash: Hasher> Deref for ResumableChunk<'a, Hash> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.chunk.deref()
    }
}

pub enum TreeNode<'a, Hash: Hasher> {
    Internal(Box<[Self]>),
    Leaf(Box<[ResumableChunk<'a, Hash>]>),
}

pub struct Tree<'a, Hash: Hasher> {
    pub root: Box<TreeNode<'a, Hash>>,
}
