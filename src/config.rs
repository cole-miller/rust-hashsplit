use crate::iter::{Delimited, Distances};
#[allow(unused)]
use crate::util::*;
use crate::{Hasher, Named};

use core::fmt;

#[derive(Clone, Copy, Default)]
pub struct Config<Hash, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize> {
    pub hasher: Hash,
}

impl<Hash: Hasher, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize>
    Config<Hash, THRESHOLD, MIN_SIZE, MAX_SIZE>
{
    pub fn new(hasher: Hash) -> Self {
        Self { hasher }
    }

    pub fn delimited<Source: Iterator<Item = u8>>(
        self,
        source: Source,
    ) -> Delimited<Hash, Source, THRESHOLD, MIN_SIZE, MAX_SIZE> {
        Delimited::start(self.hasher, source)
    }

    pub fn distances<Source: Iterator<Item = u8>>(
        self,
        source: Source,
    ) -> Distances<Hash, Source, THRESHOLD, MIN_SIZE, MAX_SIZE> {
        Distances::start(self.hasher, source)
    }
}

struct Size(usize);

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let z = self.0;

        if z % (1 << 30) == 0 {
            write!(f, "{}Gi", z >> 30)
        } else if z % (1 << 20) == 0 {
            write!(f, "{}Mi", z >> 20)
        } else if z % (1 << 10) == 0 {
            write!(f, "{}Ki", z >> 10)
        } else {
            write!(f, "{}", z)
        }
    }
}

/// ```
/// # use hashsplit::Config;
/// use hashsplit::algorithms::Rrs1;
///
/// let cfg: Config<Rrs1, 13, 0x01_00_00, 0x20_00_00> = Default::default();
///
/// assert_eq!("HashSplit_13_RRS1_64Ki_2Mi", cfg.to_string());
/// ```
impl<Hash: Named, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize> fmt::Display
    for Config<Hash, THRESHOLD, MIN_SIZE, MAX_SIZE>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "HashSplit_{}_{}_{}_{}",
            THRESHOLD,
            Hash::NAME,
            Size(MIN_SIZE),
            Size(MAX_SIZE)
        )
    }
}
