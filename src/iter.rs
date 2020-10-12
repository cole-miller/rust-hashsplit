use crate::util::*;
#[cfg(feature = "alloc")]
use crate::ResumableChunk;
use crate::{Hasher, Leveled, WINDOW_SIZE};

#[cfg(feature = "alloc")]
use alloc::{borrow::Cow, vec::Vec};

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
    type Item = Hash::Checksum;

    fn next(&mut self) -> Option<Self::Item> {
        self.source.next().map(|byte| self.feed(byte))
    }
}

pub struct WithRolling<Hash: Hasher, Source>(pub Rolling<Hash, Source>);

impl<Hash, Source> WithRolling<Hash, Source>
where
    Hash: Hasher,
{
    pub fn state(&self) -> &Hash::State {
        &self.0.state
    }
}

impl<Hash, Source> Iterator for WithRolling<Hash, Source>
where
    Hash: Hasher,
    Source: Iterator<Item = u8>,
{
    type Item = (u8, Hash::Checksum);

    fn next(&mut self) -> Option<Self::Item> {
        let rolling = &mut self.0;

        rolling.source.next().map(|byte| (byte, rolling.feed(byte)))
    }
}

pub enum Event<Hash: Hasher> {
    Data(u8),
    Boundary(u32, Hash::State),
    Capped(Hash::State),
    Eof(Hash::State),
}

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<Hash> Event<Hash>
where
    Hash: Hasher,
{
    fn collapse(self) -> Either<u8, Hash::State> {
        match self {
            Event::Data(byte) => Either::Left(byte),
            Event::Boundary(_, state) => Either::Right(state),
            Event::Capped(state) => Either::Right(state),
            Event::Eof(state) => Either::Right(state),
        }
    }
}

pub struct Delimited<
    Hash: Hasher,
    Source,
    const THRESHOLD: u32,
    const MIN_SIZE: usize,
    const MAX_SIZE: usize,
> {
    prepared: Option<(Option<u32>, Hash::State)>,
    counter: usize,
    halt: bool,
    pub input: WithRolling<Hash, Source>,
}

impl<Hash, Source, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize>
    Delimited<Hash, Source, THRESHOLD, MIN_SIZE, MAX_SIZE>
where
    Hash: Hasher,
    Source: Iterator<Item = u8>,
{
    pub fn start(hasher: Hash, source: Source) -> Self {
        Self {
            prepared: None,
            counter: 0,
            halt: false,
            input: WithRolling(Rolling::start(hasher, source)),
        }
    }

    #[cfg(feature = "alloc")]
    pub fn split(self) -> Splits<Self> {
        Splits {
            reserve: 2 * MIN_SIZE, // XXX
            preparing: None,
            source: self,
        }
    }
}

impl<Hash, Source, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize> Iterator
    for Delimited<Hash, Source, THRESHOLD, MIN_SIZE, MAX_SIZE>
where
    Hash: Hasher,
    Hash::Checksum: Leveled,
    Hash::State: Clone,
    Source: Iterator<Item = u8>,
{
    type Item = Event<Hash>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((may, state)) = self.prepared.take() {
            if let Some(lev) = may {
                return Some(Event::Boundary(lev, state));
            } else {
                return Some(Event::Capped(state));
            }
        }

        if let Some((byte, sum)) = self.input.next() {
            self.counter += 1;

            let lev = sum.level();
            if lev >= THRESHOLD && self.counter >= MIN_SIZE {
                self.prepared = Some((Some(lev), self.input.state().clone()));
            } else if self.counter == MAX_SIZE {
                self.prepared = Some((None, self.input.state().clone()));
                self.counter = 0;
            }

            return Some(Event::Data(byte));
        }

        if !self.halt {
            self.halt = true;

            return Some(Event::Eof(self.input.state().clone()));
        }

        None
    }
}

#[cfg(feature = "alloc")]
pub struct Splits<Source> {
    reserve: usize,
    preparing: Option<Vec<u8>>,
    pub source: Source,
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
            match ev.collapse() {
                Either::Left(byte) => {
                    let reserve = self.reserve;
                    self.preparing
                        .get_or_insert_with(|| Vec::with_capacity(reserve))
                        .push(byte)
                }
                Either::Right(state) => {
                    return self.preparing.take().map(|prep| ResumableChunk {
                        chunk: Cow::from(prep),
                        state,
                    });
                }
            }
        }

        None
    }
}

#[cfg(feature = "alloc")]
pub struct Spans<
    'a,
    Hash: Hasher,
    const THRESHOLD: u32,
    const MIN_SIZE: usize,
    const MAX_SIZE: usize,
> {
    counter: usize,
    saved: &'a [u8],
    delimited: Delimited<
        Hash,
        core::iter::Copied<core::slice::Iter<'a, u8>>,
        THRESHOLD,
        MIN_SIZE,
        MAX_SIZE,
    >,
}

#[cfg(feature = "alloc")]
impl<'a, Hash, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize>
    Spans<'a, Hash, THRESHOLD, MIN_SIZE, MAX_SIZE>
where
    Hash: Hasher,
{
    pub fn start(hasher: Hash, data: &'a [u8]) -> Self {
        Self {
            counter: 0,
            saved: data,
            delimited: Delimited::start(hasher, data.iter().copied()),
        }
    }

    fn reset(&mut self) -> Option<&'a [u8]> {
        (self.counter > 0).check().and_then(|_| {
            let prev = self.saved;
            self.saved = &prev[self.counter..];

            Some(&prev[..self.counter])
        })
    }
}

#[cfg(feature = "alloc")]
impl<'a, Hash, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize> Iterator
    for Spans<'a, Hash, THRESHOLD, MIN_SIZE, MAX_SIZE>
where
    Hash: Hasher,
    Hash::Checksum: Leveled,
    Hash::State: Clone,
{
    type Item = ResumableChunk<'a, Hash>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(ev) = self.delimited.next() {
            match ev.collapse() {
                Either::Left(_) => self.counter += 1,
                Either::Right(state) => {
                    return self.reset().map(|span| ResumableChunk {
                        chunk: Cow::from(span),
                        state,
                    })
                }
            }
        }

        None
    }
}
