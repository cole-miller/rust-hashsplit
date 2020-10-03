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
use core::ops::Range;

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};

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

/// An iterator adapter that pairs each byte in a sequence with the corresponding value of a chosen
/// rolling hash function.
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
    /// Constructs a `Rolling` from a given hash function, starting buffer, and input iterator.
    ///
    /// Returns `None` if `buf` converts to an empty slice.
    pub fn with_buf<Buf>(hasher: Hash, buf: Buf, source: Source) -> Option<Self>
    where
        Buf: Into<Box<[u8]>>,
    {
        let buf = buf.into();

        Some(Self {
            hasher,
            state: Hash::INITIAL_STATE,
            width: NonZeroUsize::new(buf.len())?,
            begin: 0,
            ring: buf,
            source,
        })
    }

    /// Constructs a `Rolling` from a given starting buffer and input iterator, using an implicit
    /// default hash function. This is suitable for hasher types that have a trivial run-time
    /// representation.
    ///
    /// Returns `None` if `buf` converts to an empty slice.
    pub fn default_with_buf<Buf>(buf: Buf, source: Source) -> Option<Self>
    where
        Buf: Into<Box<[u8]>>,
        Hash: Default,
    {
        Self::with_buf(Default::default(), buf, source)
    }

    /// Constructs a `Rolling` from a given hash function, buffer width, and input iterator, using
    /// a buffer initialized to all zeros.
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

    /// Constructs a `Rolling` from a given buffer width and input iterator, using an implicit
    /// default hash function and a buffer initialized to all zeros.
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

    pub fn delimited(self, threshold: u32) -> Delimited<Self>
    where
        Hash::Checksum: Leveled,
    {
        Delimited::start(threshold, self)
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

/// A value that is either a piece of data or a boundary (with associated significance level)
/// between runs of data in a sequence.
pub enum Event<T> {
    Data(T),
    Boundary(u32),
}

/// An iterator adapter that transforms a sequence of data/checksum pairs into a sequence of data
/// interspersed with boundary markers where the checksum value achieves a given "significance
/// level".
///
/// The `Iterator` implementation for this type guarantees that an `Event::Boundary` can only be
/// yielded immediately after an `Event::Data`.
pub struct Delimited<Source> {
    threshold: u32,
    prepared: Option<u32>,
    /// The input iterator.
    pub source: Source,
}

impl<Source, T, U> Delimited<Source>
where
    Source: Iterator<Item = (T, U)>,
    U: Leveled,
{
    /// Construct a `Delimited` from a threshold significance level and an input iterator.
    pub fn start(threshold: u32, source: Source) -> Self {
        Self {
            threshold,
            prepared: None,
            source,
        }
    }

    #[cfg(feature = "alloc")]
    pub fn splits(self, reserve: usize) -> Splits<Self> {
        Splits::start(reserve, self)
    }

    pub fn spans(self) -> Spans<Self> {
        Spans::start(self)
    }
}

impl<Source, T, U> Iterator for Delimited<Source>
where
    Source: Iterator<Item = (T, U)>,
    U: Leveled,
{
    type Item = Event<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(lev) = self.prepared.take() {
            return Some(Event::Boundary(lev));
        }

        self.source.next().map(|(dat, sum)| {
            let lev = sum.level();

            if lev >= self.threshold {
                self.prepared = Some(lev);
            }

            Event::Data(dat)
        })
    }
}

#[doc(hidden)]
pub trait SomeEvent {
    type Param;
}

impl<T> SomeEvent for Event<T> {
    type Param = T;
}

/// An iterator adapter that transforms a sequence of data interspersed with boundaries into a
/// sequence of owned slices that collect the runs of data between successive boundaries.
///
/// The `SomeEvent` constraint on `Source::Item` allows extracting the type parameter `T` from the
/// generic type `Event<T>`, without introducing a superfluous type parameter on this struct.
/// Although `SomeEvent` is `pub`, as required to write this constraint, it should not be used
/// outside this module, and is marked `#[doc(hidden)]` to emphasize this. All that's guaranteed is
/// that `Event<T>` implements `SomeEvent` for every (sized) `T`.
#[cfg(feature = "alloc")]
pub struct Splits<Source>
where
    Source: Iterator,
    Source::Item: SomeEvent,
{
    reserve: usize,
    // hack to avoid declaring a second type parameter
    preparing: Option<Vec<<<Source as Iterator>::Item as SomeEvent>::Param>>,
    halt: bool,
    /// The input iterator.
    pub source: Source,
}

#[cfg(feature = "alloc")]
impl<Source, T> Splits<Source>
where
    // note no `SomeEvent` here
    Source: Iterator<Item = Event<T>>,
{
    pub fn start(reserve: usize, source: Source) -> Self {
        Self {
            reserve,
            preparing: None,
            halt: false,
            source,
        }
    }

    fn yield_prepared(&mut self) -> Option<Box<[T]>> {
        self.preparing.take().map(Vec::into_boxed_slice)
    }
}

#[cfg(feature = "alloc")]
impl<Source, T> Iterator for Splits<Source>
where
    Source: Iterator<Item = Event<T>>,
{
    type Item = Box<[T]>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.halt {
            return None;
        }

        while let Some(ev) = self.source.next() {
            match ev {
                Event::Data(dat) => {
                    let reserve = self.reserve;

                    self.preparing
                        // lazy insertion to avoid allocating every time
                        .get_or_insert_with(|| Vec::with_capacity(reserve))
                        .push(dat);
                }
                Event::Boundary(_) => {
                    return self.yield_prepared();
                }
            }
        }

        self.halt = true;
        self.yield_prepared()
    }
}

/// An iterator adapter that transforms a sequence of data interspersed with boundaries into a
/// sequence of index ranges that describe the runs of data between successive boundaries.
pub struct Spans<Source> {
    next_ix: usize,
    base_ix: usize,
    halt: bool,
    /// The input iterator.
    pub source: Source,
}

impl<Source> Spans<Source> {
    pub fn start(source: Source) -> Self {
        Self {
            next_ix: 0,
            base_ix: 0,
            halt: false,
            source,
        }
    }
}

impl<Source, T> Iterator for Spans<Source>
where
    Source: Iterator<Item = Event<T>>,
{
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.halt {
            return None;
        }

        while let Some(ev) = self.source.next() {
            match ev {
                Event::Data(_) => {
                    self.next_ix += 1;
                }
                Event::Boundary(_) => {
                    let saved_ix = core::mem::replace(&mut self.base_ix, self.next_ix);

                    return Some(saved_ix..self.next_ix);
                }
            }
        }

        self.halt = true;
        (self.next_ix > self.base_ix).check().and_then(|_| {
            let saved_ix = core::mem::replace(&mut self.base_ix, self.next_ix);

            Some(saved_ix..self.next_ix)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct TrivialHasher;

    impl Hasher for TrivialHasher {
        type Checksum = u8;

        type State = ();

        const INITIAL_STATE: Self::State = ();

        fn process_byte(
            &self,
            _state: Self::State,
            _width: NonZeroUsize,
            _old_byte: u8,
            new_byte: u8,
        ) -> (Self::Checksum, Self::State) {
            (new_byte, ())
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

pub mod thinned;
pub mod rrs;
