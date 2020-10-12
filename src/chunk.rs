use crate::Hasher;

use alloc::{borrow::Cow, boxed::Box};

pub struct ResumableChunk<'a, Hash: Hasher> {
    pub chunk: Cow<'a, [u8]>,
    pub state: Hash::State,
}

pub enum TreeNode<'a, Hash: Hasher> {
    Internal(Box<[Self]>),
    Leaf(Box<[ResumableChunk<'a, Hash>]>),
}

pub struct Tree<'a, Hash: Hasher> {
    pub root: Box<TreeNode<'a, Hash>>,
}
