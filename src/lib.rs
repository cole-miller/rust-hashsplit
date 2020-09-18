#![no_std]

#[cfg(feature = "std")]
extern crate std;

use util::*;

use core::num::NonZeroUsize;

#[cfg(feature = "std")]
use std::vec::Vec;

pub trait Leveled {
    fn level(&self) -> u32;
}

macro_rules! implement_leveled_for_primitive {
    ($t:ty) => {
        impl $crate::Leveled for $t {
            fn level(&self) -> u32 {
                self.trailing_zeros()
            }
        }
    };
}

implement_leveled_for_primitive! {i8}
implement_leveled_for_primitive! {i16}
implement_leveled_for_primitive! {i32}
implement_leveled_for_primitive! {i64}
implement_leveled_for_primitive! {i128}
implement_leveled_for_primitive! {isize}

implement_leveled_for_primitive! {u8}
implement_leveled_for_primitive! {u16}
implement_leveled_for_primitive! {u32}
implement_leveled_for_primitive! {u64}
implement_leveled_for_primitive! {u128}
implement_leveled_for_primitive! {usize}

pub trait Hasher {
    type Checksum: Leveled;

    type State;

    const EMPTY_CHECKSUM: Self::Checksum;

    const INITIAL_STATE: Self::State;

    fn width(&self) -> NonZeroUsize;

    fn process_byte(
        &self,
        state: Self::State,
        old_byte: u8,
        new_byte: u8,
    ) -> (Self::Checksum, Self::State);

    fn process_slice(
        &self,
        state: Self::State,
        old_data: &[u8],
        new_data: &[u8],
    ) -> (Self::Checksum, Self::State) {
        old_data.iter().zip(new_data.iter()).fold(
            (Self::EMPTY_CHECKSUM, state),
            |(_, prev_state), (old_byte, new_byte)| {
                self.process_byte(prev_state, *old_byte, *new_byte)
            },
        )
    }
}

pub trait Grain<Block>: Hasher
where
    Block: AsRef<[u8]>,
{
    fn process_block(
        &self,
        state: Self::State,
        old_data: &Block,
        new_data: &Block,
    ) -> (Self::Checksum, Self::State) {
        self.process_slice(state, old_data.as_ref(), new_data.as_ref())
    }
}

#[cfg(feature = "std")]
pub struct Rolling<H, I>
where
    H: Hasher,
{
    hasher: H,
    state: H::State,
    begin: usize,
    ring: Vec<u8>,
    bytes: I,
}

#[cfg(feature = "std")]
impl<H, I> Rolling<H, I>
where
    H: Hasher,
    I: Iterator<Item = u8>,
{
    pub fn try_start(hasher: H, mut it: I) -> Option<Self> {
        let width = hasher.width().get();
        let mut ring = Vec::with_capacity(width);

        for byte in &mut it {
            ring.push(byte);
            if ring.len() == width {
                break;
            }
        }

        (ring.len() == width).and_some(Self {
            hasher,
            state: H::INITIAL_STATE,
            begin: 0,
            ring,
            bytes: it,
        })
    }

    fn feed(&mut self, byte: u8) -> H::Checksum {
        let mut dummy = H::INITIAL_STATE;
        core::mem::swap(&mut dummy, &mut self.state);
        let prev_state = dummy;

        let (sum, new_state) = self
            .hasher
            .process_byte(prev_state, self.ring[self.begin], byte);
        self.state = new_state;
        self.ring[self.begin] = byte;
        self.begin += 1;
        if self.begin == self.hasher.width().get() {
            self.begin = 0;
        }

        sum
    }
}

#[cfg(feature = "std")]
impl<H, I> Iterator for Rolling<H, I>
where
    H: Hasher,
    I: Iterator<Item = u8>,
{
    type Item = H::Checksum;

    fn next(&mut self) -> Option<Self::Item> {
        self.bytes.next().map(|byte| self.feed(byte))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct TrivialHasher;

    impl Hasher for TrivialHasher {
        type Checksum = u8;

        type State = ();

        const EMPTY_CHECKSUM: Self::Checksum = 0;

        const INITIAL_STATE: Self::State = ();

        fn width(&self) -> NonZeroUsize {
            NonZeroUsize::new(1).unwrap()
        }

        fn process_byte(
            &self,
            _state: Self::State,
            _old_byte: u8,
            new_byte: u8,
        ) -> (Self::Checksum, Self::State) {
            (new_byte, ())
        }
    }
}

pub(crate) mod util {
    pub trait SwitchOption {
        fn and_some<T>(&self, x: T) -> Option<T>;

        fn and_then_some<T, F: FnOnce() -> T>(&self, f: F) -> Option<T>;
    }

    impl SwitchOption for bool {
        fn and_some<T>(&self, x: T) -> Option<T> {
            if *self {
                Some(x)
            } else {
                None
            }
        }

        fn and_then_some<T, F: FnOnce() -> T>(&self, f: F) -> Option<T> {
            if *self {
                Some(f())
            } else {
                None
            }
        }
    }
}

pub mod rrs;
