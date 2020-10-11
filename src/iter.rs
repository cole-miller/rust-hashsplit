use crate::util::*;
use crate::ResumableChunk;
use crate::{Event, Hasher, Leveled, Rolling};

use alloc::{borrow::Cow, vec::Vec};

use either::Either;

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
    pub rolling: Rolling<Hash, Source>,
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
            rolling: Rolling::start(hasher, source),
        }
    }

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

        if let Some((byte, sum)) = self.rolling.next() {
            self.counter += 1;

            let lev = sum.level();
            if lev >= THRESHOLD && self.counter >= MIN_SIZE {
                self.prepared = Some((Some(lev), self.rolling.state.clone()));
            } else if self.counter == MAX_SIZE {
                self.prepared = Some((None, self.rolling.state.clone()));
                self.counter = 0;
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

pub struct Splits<Source> {
    reserve: usize,
    preparing: Option<Vec<u8>>,
    pub source: Source,
}

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
