#![no_std]
#![feature(min_const_generics)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::borrow::Cow;
use either::Either;

pub const WINDOW_SIZE: usize = 64;

pub trait Leveled {
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
        old_data: &[u8],
        new_data: &[u8],
    ) -> (Self::Checksum, Self::State) {
        old_data.iter().copied().zip(new_data.iter().copied()).fold(
            (Default::default(), state),
            |(_, prev_state), (old_byte, new_byte)| {
                self.process_byte(prev_state, old_byte, new_byte)
            },
        )
    }
}
pub trait Named: Hasher {
    const NAME: &'static str;
}

pub struct Rolling<Hash: Hasher, Source> {
    hasher: Hash,
    state: Hash::State,
    begin: usize,
    ring: [u8; WINDOW_SIZE],
    /// The input iterator.
    pub source: Source,
}

impl<Hash, Source> Rolling<Hash, Source>
where
    Hash: Hasher,
    Source: Iterator<Item = u8>,
{
    pub fn start(hasher: Hash, source: Source) -> Self {
        Self {
            hasher,
            state: Hash::INITIAL_STATE,
            begin: 0,
            ring: [0; WINDOW_SIZE],
            source,
        }
    }

    fn feed(&mut self, byte: u8) -> Hash::Checksum {
        let prev_state = core::mem::replace(&mut self.state, Hash::INITIAL_STATE);

        let (sum, new_state) = self
            .hasher
            .process_byte(prev_state, self.ring[self.begin], byte);
        self.state = new_state;
        self.ring[self.begin] = byte;
        self.begin += 1;
        if self.begin == WINDOW_SIZE {
            self.begin = 0;
        }

        sum
    }
}

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


#[cfg(feature = "alloc")]
pub struct ResumableChunk<'a, Hash: Hasher> {
    pub chunk: Cow<'a, [u8]>,
    pub state: Hash::State,
}

pub enum Event<Hash: Hasher> {
    Data(u8),
    Boundary(u32, Hash::State),
    Capped(Hash::State),
    Eof(Hash::State),
}

impl<Hash> Event<Hash>
where
    Hash: Hasher,
{
    pub(crate) fn collapse(self) -> Either<u8, Hash::State> {
        match self {
            Event::Data(byte) => Either::Left(byte),
            Event::Boundary(_, state) => Either::Right(state),
            Event::Capped(state) => Either::Right(state),
            Event::Eof(state) => Either::Right(state),
        }
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

pub mod algorithms;
pub mod config;
#[cfg(feature = "alloc")]
pub mod iter;
pub mod thin;
#[cfg(feature = "alloc")]
pub mod tree;
