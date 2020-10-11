#[cfg(feature = "alloc")]
use crate::iter::{Delimited, Spans};
use crate::{Hasher, Named};

#[derive(Default)]
pub struct Config<Hash, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize> {
    pub hasher: Hash,
}

impl<Hash, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize>
    Config<Hash, THRESHOLD, MIN_SIZE, MAX_SIZE>
where
    Hash: Hasher,
{
    pub fn new(hasher: Hash) -> Self {
        Self { hasher }
    }

    #[cfg(feature = "alloc")]
    pub fn delimited<Source: Iterator<Item = u8>>(
        self,
        source: Source,
    ) -> Delimited<Hash, Source, THRESHOLD, MIN_SIZE, MAX_SIZE> {
        Delimited::start(self.hasher, source)
    }

    #[cfg(feature = "alloc")]
    pub fn spans(self, data: &[u8]) -> Spans<'_, Hash, THRESHOLD, MIN_SIZE, MAX_SIZE> {
        Spans::start(self.hasher, data)
    }
}

struct Size(usize);

impl core::fmt::Display for Size {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
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

impl<Hash, const THRESHOLD: u32, const MIN_SIZE: usize, const MAX_SIZE: usize> core::fmt::Display
    for Config<Hash, THRESHOLD, MIN_SIZE, MAX_SIZE>
where
    Hash: Named,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::rrs::Rrs1;

    use alloc::string::ToString;

    #[test]
    fn display_basic_config() {
        let x: Config<Rrs1, 13, 65_536, 2_097_152> = Default::default();
        assert_eq!("HashSplit_13_RRS1_64Ki_2Mi", x.to_string())
    }
}
