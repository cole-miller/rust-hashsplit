/*!
This library provides traits and types for hashsplitting and incremental hashing more generally.

*Incremental hashing* is the use of hash functions that assign a checksum to each item in a
sequence, with the property that after localized change in the sequence, these checksums will
remain unaltered outside a bounded neighborhood of the change. Such a hash function is often called
a "rolling hash function".

*Hashsplitting* is one name for the technique of breaking a sequence of items into pieces — usually
called "chunks" — based on their associated checksum values. Hashsplitting becomes useful in
combination with incremental hashing, since a change in the sequence of items will not require
re-splitting the whole sequence, but only the neighborhood given by our rolling hash function. This
is in contrast to splitting the sequence after every `N`th item, for example.
*/

#![no_std]
#![feature(min_const_generics)]

#[cfg(feature = "alloc")]
extern crate alloc;

use crate::util::*;

use core::num::NonZeroUsize;
use core::ops::{Deref, Range};

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "immutable-vector")]
use im_rc as im;

/// Describes the required behavior for values of a hash function that's used for splitting.
pub trait Leveled {
    /// Return the "significance level" for this checksum value.
    fn level(self) -> u32;
}

impl Leveled for bool {
    fn level(self) -> u32 {
        !self as u32
    }
}

macro_rules! implement_leveled_for_integer_primitive {
    ($t:ty) => {
        impl $crate::Leveled for $t {
            fn level(self) -> u32 {
                self.trailing_zeros()
            }
        }
    };
}

implement_leveled_for_integer_primitive! {i128}
implement_leveled_for_integer_primitive! {i16}
implement_leveled_for_integer_primitive! {i32}
implement_leveled_for_integer_primitive! {i64}
implement_leveled_for_integer_primitive! {i8}

implement_leveled_for_integer_primitive! {u128}
implement_leveled_for_integer_primitive! {u16}
implement_leveled_for_integer_primitive! {u32}
implement_leveled_for_integer_primitive! {u64}
implement_leveled_for_integer_primitive! {u8}

/// Defines the interface expected from each rolling hash function.
///
/// All hashsplitting operations are generic over this trait.
pub trait Hasher {
    /// The type of values returned by this hash function.
    type Checksum: Default;

    /// The type of data carried between successive invocations of this hash function during
    /// rolling computation.
    type State;

    /// Starting value of the state data for rolling computations.
    const INITIAL_STATE: Self::State;

    /// Specify how to compute this hash function.
    ///
    /// A rolling hash function takes as input some state data, the value of the byte passing out
    /// of its window, and the value of the byte entering its window, and returns a checksum along
    /// with new state data to be propagated to the next invocation.
    fn process_byte(
        &self,
        state: Self::State,
        width: NonZeroUsize,
        old_byte: u8,
        new_byte: u8,
    ) -> (Self::Checksum, Self::State);

    /// Compute this rolling hash function for each byte in a contiguous range, returning only the
    /// final checksum and state data.
    ///
    /// If this method is overriden by an implementor, the overriding definition must return the
    /// same values as the provided definition for identical inputs.
    fn process_slice(
        &self,
        state: Self::State,
        width: NonZeroUsize,
        old_data: &[u8],
        new_data: &[u8],
    ) -> (Self::Checksum, Self::State) {
        old_data.iter().copied().zip(new_data.iter().copied()).fold(
            (Default::default(), state),
            |(_, prev_state), (old_byte, new_byte)| {
                self.process_byte(prev_state, width, old_byte, new_byte)
            },
        )
    }
}

#[cfg(feature = "alloc")]
pub struct Rolling<Hash, Source>
where
    Hash: Hasher,
{
    hasher: Hash,
    state: Hash::State,
    width: NonZeroUsize,
    begin: usize,
    ring: Box<[u8]>,
    /// The input iterator.
    pub source: Source,
}

#[cfg(feature = "alloc")]
impl<Hash, Source> Rolling<Hash, Source>
where
    Hash: Hasher,
    Source: Iterator<Item = u8>,
{
    pub fn with_buf(hasher: Hash, buf: Box<[u8]>, source: Source) -> Option<Self> {
        Some(Self {
            hasher,
            state: Hash::INITIAL_STATE,
            width: NonZeroUsize::new(buf.len())?,
            begin: 0,
            ring: buf,
            source,
        })
    }

    pub fn default_with_buf(buf: Box<[u8]>, source: Source) -> Option<Self>
    where
        Hash: Default,
    {
        Self::with_buf(Default::default(), buf, source)
    }

    pub fn with_zeros(hasher: Hash, width: NonZeroUsize, source: Source) -> Self {
        Self {
            hasher,
            state: Hash::INITIAL_STATE,
            width,
            begin: 0,
            ring: alloc::vec![0; width.get()].into_boxed_slice(),
            source,
        }
    }

    pub fn default_with_zeros(width: NonZeroUsize, source: Source) -> Self
    where
        Hash: Default,
    {
        Self::with_zeros(Default::default(), width, source)
    }

    fn feed(&mut self, byte: u8) -> Hash::Checksum {
        let prev_state = core::mem::replace(&mut self.state, Hash::INITIAL_STATE);

        let (sum, new_state) =
            self.hasher
                .process_byte(prev_state, self.width, self.ring[self.begin], byte);
        self.state = new_state;
        self.ring[self.begin] = byte;
        self.begin += 1;
        if self.begin == self.width.get() {
            self.begin = 0;
        }

        sum
    }
}

#[cfg(feature = "alloc")]
impl<Hash, Source> Iterator for Rolling<Hash, Source>
where
    Hash: Hasher,
    Source: Iterator<Item = u8>,
{
    type Item = (u8, Hash::Checksum);

    fn next(&mut self) -> Option<Self::Item> {
        self.source.next().map(|byte| (byte, self.feed(byte)))
    }
}

pub enum Event<Hash: Hasher> {
    Data(u8),
    Boundary(u32, Hash::State),
    Eof(Hash::State),
}

#[cfg(feature = "alloc")]
pub struct Delimited<Hash: Hasher, Source> {
    threshold: u32,
    prepared: Option<(u32, Hash::State)>,
    halt: bool,
    pub rolling: Rolling<Hash, Source>,
}

#[cfg(feature = "alloc")]
impl<Hash, Source> Iterator for Delimited<Hash, Source>
where
    Hash: Hasher,
    Hash::Checksum: Leveled,
    Hash::State: Clone,
    Source: Iterator<Item = u8>,
{
    type Item = Event<Hash>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((lev, state)) = self.prepared.take() {
            return Some(Event::Boundary(lev, state));
        }

        if let Some((byte, sum)) = self.rolling.next() {
            let lev = sum.level();
            if lev >= self.threshold {
                self.prepared = Some((lev, self.rolling.state.clone()));
            }

            return Some(Event::Data(byte));
        }

        if !self.halt {
            self.halt = true;

            return Some(Event::Eof(self.rolling.state.clone()));
        }

        None
    }
}

#[cfg(feature = "alloc")]
pub enum LiveBytes<'a> {
    Borrowing(&'a [u8]),
    Owning(Box<[u8]>),
}

#[cfg(feature = "alloc")]
impl<'a> Deref for LiveBytes<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowing(x) => x,
            Self::Owning(ref x) => x,
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a> AsRef<[u8]> for LiveBytes<'a> {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

#[cfg(feature = "alloc")]
pub struct ResumableChunk<'a, Hash: Hasher> {
    chunk: LiveBytes<'a>,
    state: Hash::State,
}

#[cfg(feature = "alloc")]
pub struct Splits<Source> {
    reserve: usize,
    preparing: Option<Vec<u8>>,
    pub source: Source,
}

#[cfg(feature = "alloc")]
impl<Source> Splits<Source> {
    fn yield_prepared(&mut self) -> Option<Box<[u8]>> {
        self.preparing.take().map(Vec::into_boxed_slice)
    }
}

#[cfg(feature = "alloc")]
impl<Hash, Source> Iterator for Splits<Source>
where
    Hash: Hasher,
    Source: Iterator<Item = Event<Hash>>,
{
    type Item = ResumableChunk<'static, Hash>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(ev) = self.source.next() {
            match ev {
                Event::Data(byte) => {
                    let reserve = self.reserve;
                    self.preparing
                        .get_or_insert_with(|| Vec::with_capacity(reserve))
                        .push(byte)
                }
                Event::Boundary(_, state) => {
                    return self.yield_prepared().map(|prep| ResumableChunk {
                        chunk: LiveBytes::Owning(prep),
                        state,
                    });
                }
                Event::Eof(state) => {
                    return self.yield_prepared().map(|prep| ResumableChunk {
                        chunk: LiveBytes::Owning(prep),
                        state,
                    });
                }
            }
        }

        None
    }
}

pub struct Spans<Source> {
    base_ix: usize,
    next_ix: usize,
    source: Source,
}

impl<Source> Spans<Source> {
    fn yield_prepared(&mut self) -> Option<Range<usize>> {
        if self.next_ix > self.base_ix {
            let saved_ix = self.base_ix;
            self.base_ix = self.next_ix;

            Some(saved_ix..self.next_ix)
        } else {
            None
        }
    }
}

impl<Hash, Source> Iterator for Spans<Source>
where
    Hash: Hasher,
    Source: Iterator<Item = Event<Hash>>,
{
    type Item = (Range<usize>, Hash::State);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(ev) = self.source.next() {
            match ev {
                Event::Data(_) => self.next_ix += 1,
                Event::Boundary(_, state) => {
                    return self.yield_prepared().map(|prep| (prep, state))
                }
                Event::Eof(state) => return self.yield_prepared().map(|prep| (prep, state)),
            }
        }

        None
    }
}

pub(crate) mod util {
    pub trait Checkpoint {
        fn check(self) -> Option<()>;
    }

    impl Checkpoint for bool {
        fn check(self) -> Option<()> {
            if self {
                Some(())
            } else {
                None
            }
        }
    }
}

pub mod rrs;
pub mod thin;
#[cfg(feature = "alloc")]
pub mod tree;
