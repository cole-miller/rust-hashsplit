use crate::{Hasher, ResumableChunk};

use alloc::boxed::Box;

pub enum TreeNode<'a, Hash: Hasher> {
    Internal(Box<[Self]>),
    Leaf(Box<[ResumableChunk<'a, Hash>]>),
}

pub struct Tree<'a, Hash: Hasher> {
    pub root: Box<TreeNode<'a, Hash>>,
}
