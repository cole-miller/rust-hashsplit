#[cfg(feature = "alloc")]
use crate::chunk::ResumableChunk;
#[allow(unused)]
use crate::util::*;
use crate::{Hasher, Leveled, WINDOW_SIZE};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::num::NonZeroUsize;

pub struct Rolling<Hash: Hasher, Source> {
    hasher: Hash,
    state: Hash::State,
    begin: usize,
    ring: [u8; WINDOW_SIZE],
    pub source: Source,
}

impl<Hash: Hasher, Source: Iterator<Item = u8>> Rolling<Hash, Source> {
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

impl<Hash: Hasher, Source: Iterator<Item = u8>> Iterator for Rolling<Hash, Source> {
    type Item = Hash::Checksum;

    fn next(&mut self) -> Option<Self::Item> {
        self.source.next().map(|byte| self.feed(byte))
    }
}

pub struct WithRolling<Hash: Hasher, Source>(pub Rolling<Hash, Source>);

impl<Hash: Hasher, Source> WithRolling<Hash, Source> {
    pub fn state(&self) -> &Hash::State {
        &self.0.state
    }
}

impl<Hash: Hasher, Source: Iterator<Item = u8>> Iterator for WithRolling<Hash, Source> {
    type Item = (u8, Hash::Checksum);

    fn next(&mut self) -> Option<Self::Item> {
        let rolling = &mut self.0;

        rolling.source.next().map(|byte| (byte, rolling.feed(byte)))
    }
}

pub enum Boundary<Hash: Hasher> {
    Level(u32, Hash::State),
    Capped(Hash::State),
    Eof(Hash::State),
}

impl<Hash: Hasher> Boundary<Hash> {
    pub fn into_state(self) -> Hash::State {
        match self {
            Self::Level(_, state) => state,
            Self::Capped(state) => state,
            Self::Eof(state) => state,
        }
    }
}

pub enum Event<Hash: Hasher> {
    Data(u8),
    Boundary(Boundary<Hash>),
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

impl<
        Hash: Hasher,
        Source: Iterator<Item = u8>,
        const THRESHOLD: u32,
        const MIN_SIZE: usize,
        const MAX_SIZE: usize,
    > Delimited<Hash, Source, THRESHOLD, MIN_SIZE, MAX_SIZE>
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
    pub fn splits(self) -> Splits<Self> {
        Splits {
            reserve: 2 * MIN_SIZE, // XXX
            preparing: None,
            source: self,
        }
    }
}

impl<
        Hash: Hasher,
        Source: Iterator<Item = u8>,
        const THRESHOLD: u32,
        const MIN_SIZE: usize,
        const MAX_SIZE: usize,
    > Iterator for Delimited<Hash, Source, THRESHOLD, MIN_SIZE, MAX_SIZE>
where
    Hash::Checksum: Leveled,
    Hash::State: Clone,
{
    type Item = Event<Hash>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.halt {
            return None;
        }

        if let Some((may, state)) = self.prepared.take() {
            return Some(Event::Boundary(if let Some(lev) = may {
                Boundary::Level(lev, state)
            } else {
                Boundary::Capped(state)
            }));
        }

        if let Some((byte, sum)) = self.input.next() {
            self.counter += 1;

            let lev = sum.level();
            if lev >= THRESHOLD && self.counter >= MIN_SIZE {
                self.prepared = Some((Some(lev), self.input.state().clone()));
                self.counter = 0;
            } else if self.counter == MAX_SIZE {
                self.prepared = Some((None, self.input.state().clone()));
                self.counter = 0;
            }

            return Some(Event::Data(byte));
        }

        self.halt = true;

        Some(Event::Boundary(Boundary::Eof(self.input.state().clone())))
    }
}

#[cfg(feature = "alloc")]
#[doc(cfg(feature = "alloc"))]
pub struct Splits<Source> {
    reserve: usize,
    preparing: Option<Vec<u8>>,
    pub source: Source,
}

#[cfg(feature = "alloc")]
#[doc(cfg(feature = "alloc"))]
impl<Hash: Hasher, Source: Iterator<Item = Event<Hash>>> Iterator for Splits<Source> {
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
                Event::Boundary(bd) => {
                    return self
                        .preparing
                        .take()
                        .map(|prep| ResumableChunk::new(prep, bd.into_state()));
                }
            }
        }

        None
    }
}

pub struct Extent<Hash: Hasher> {
    pub length: NonZeroUsize,
    pub boundary: Boundary<Hash>,
}

pub struct Distances<
    Hash: Hasher,
    Source,
    const THRESHOLD: u32,
    const MIN_SIZE: usize,
    const MAX_SIZE: usize,
> {
    counter: usize,
    halt: bool,
    pub input: Rolling<Hash, Source>,
}

impl<
        Hash: Hasher,
        Source: Iterator<Item = u8>,
        const THRESHOLD: u32,
        const MIN_SIZE: usize,
        const MAX_SIZE: usize,
    > Distances<Hash, Source, THRESHOLD, MIN_SIZE, MAX_SIZE>
{
    pub fn start(hasher: Hash, source: Source) -> Self {
        Self {
            counter: 0,
            halt: false,
            input: Rolling::start(hasher, source),
        }
    }

    fn yield_extent(&mut self, boundary: Boundary<Hash>) -> Option<Extent<Hash>> {
        Some(Extent {
            length: NonZeroUsize::new(core::mem::replace(&mut self.counter, 0))?,
            boundary,
        })
    }
}

impl<
        Hash: Hasher,
        Source: Iterator<Item = u8>,
        const THRESHOLD: u32,
        const MIN_SIZE: usize,
        const MAX_SIZE: usize,
    > Iterator for Distances<Hash, Source, THRESHOLD, MIN_SIZE, MAX_SIZE>
where
    Hash::Checksum: Leveled,
    Hash::State: Clone,
{
    type Item = Extent<Hash>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.halt {
            return None;
        }

        while let Some(sum) = self.input.next() {
            self.counter += 1;

            let lev = sum.level();
            if lev >= THRESHOLD && self.counter >= MIN_SIZE {
                return self.yield_extent(Boundary::Level(lev, self.input.state.clone()));
            } else if self.counter == MAX_SIZE {
                return self.yield_extent(Boundary::Capped(self.input.state.clone()));
            }
        }

        self.halt = true;

        self.yield_extent(Boundary::Eof(self.input.state.clone()))
    }
}

#[cfg(feature = "alloc")]
#[doc(cfg(feature = "alloc"))]
pub struct Spans<
    'a,
    Hash: Hasher,
    const THRESHOLD: u32,
    const MIN_SIZE: usize,
    const MAX_SIZE: usize,
> {
    saved: &'a [u8],
    distances: Distances<
        Hash,
        core::iter::Copied<core::slice::Iter<'a, u8>>,
        THRESHOLD,
        MIN_SIZE,
        MAX_SIZE,
    >,
}

#[cfg(feature = "alloc")]
#[doc(cfg(feature = "alloc"))]
impl<'a, Hash: Hasher, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize>
    Spans<'a, Hash, THRESHOLD, MIN_SIZE, MAX_SIZE>
{
    pub fn start(hasher: Hash, data: &'a [u8]) -> Self {
        Self {
            saved: data,
            distances: Distances::start(hasher, data.iter().copied()),
        }
    }
}

#[cfg(feature = "alloc")]
#[doc(cfg(feature = "alloc"))]
impl<'a, Hash: Hasher, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize> Iterator
    for Spans<'a, Hash, THRESHOLD, MIN_SIZE, MAX_SIZE>
where
    Hash::Checksum: Leveled,
    Hash::State: Clone,
{
    type Item = ResumableChunk<'a, Hash>;

    fn next(&mut self) -> Option<Self::Item> {
        self.distances.next().map(|Extent { length, boundary }| {
            let prev = self.saved;
            self.saved = &prev[length.get()..];

            ResumableChunk::new(&prev[..length.get()], boundary.into_state())
        })
    }
}
