#![no_std]

#[cfg(feature = "std")]
extern crate std;

use util::*;

use arrayref::array_refs;
use core::borrow::Borrow;

#[cfg(feature = "std")]
use std::vec::Vec;

pub trait TrailingZeros {
    fn count_trailing_zeros(&self) -> u32;
}

macro_rules! implement_trailing_zeros_for_primitive {
    ($t:ty) => {
        impl $crate::TrailingZeros for $t {
            fn count_trailing_zeros(&self) -> u32 {
                self.trailing_zeros()
            }
        }
    };
}

implement_trailing_zeros_for_primitive! {i8}
implement_trailing_zeros_for_primitive! {i16}
implement_trailing_zeros_for_primitive! {i32}
implement_trailing_zeros_for_primitive! {i64}
implement_trailing_zeros_for_primitive! {i128}
implement_trailing_zeros_for_primitive! {isize}

implement_trailing_zeros_for_primitive! {u8}
implement_trailing_zeros_for_primitive! {u16}
implement_trailing_zeros_for_primitive! {u32}
implement_trailing_zeros_for_primitive! {u64}
implement_trailing_zeros_for_primitive! {u128}
implement_trailing_zeros_for_primitive! {usize}

pub trait Hasher {
    type Checksum: TrailingZeros;

    type State;

    const EMPTY_CHECKSUM: Self::Checksum;

    const INITIAL_STATE: Self::State;

    fn width(&self) -> usize;

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

    unsafe fn process_chunk64(
        &self,
        state: Self::State,
        old_data: &[u8; 8],
        new_data: &[u8; 8],
    ) -> (Self::Checksum, Self::State) {
        self.process_slice(state, old_data, new_data)
    }

    unsafe fn process_chunk128(
        &self,
        state: Self::State,
        old_data: &[u8; 16],
        new_data: &[u8; 16],
    ) -> (Self::Checksum, Self::State) {
        let (old_front, old_back) = array_refs![old_data, 8, 8];
        let (new_front, new_back) = array_refs![new_data, 8, 8];

        let (_, new_state) = self.process_chunk64(state, old_front, new_front);

        self.process_chunk64(new_state, old_back, new_back)
    }

    unsafe fn process_chunk256(
        &self,
        state: Self::State,
        old_data: &[u8; 32],
        new_data: &[u8; 32],
    ) -> (Self::Checksum, Self::State) {
        let (old_front, old_back) = array_refs![old_data, 16, 16];
        let (new_front, new_back) = array_refs![new_data, 16, 16];

        let (_, new_state) = self.process_chunk128(state, old_front, new_front);

        self.process_chunk128(new_state, old_back, new_back)
    }

    unsafe fn process_chunk512(
        &self,
        state: Self::State,
        old_data: &[u8; 64],
        new_data: &[u8; 64],
    ) -> (Self::Checksum, Self::State) {
        let (old_front, old_back) = array_refs![old_data, 32, 32];
        let (new_front, new_back) = array_refs![new_data, 32, 32];

        let (_, new_state) = self.process_chunk256(state, old_front, new_front);

        self.process_chunk256(new_state, old_back, new_back)
    }
}

#[cfg(feature = "std")]
pub struct Rolling<H, I>
where
    H: Hasher,
{
    hasher: H,
    next: Option<H::Checksum>,
    state: H::State,
    begin: usize,
    ring: Vec<u8>,
    bytes: I,
}

#[cfg(feature = "std")]
impl<H, I, X> Rolling<H, I>
where
    H: Hasher,
    I: Iterator<Item = X>,
    X: Borrow<u8>,
{
    pub fn start(hasher: H, mut it: I) -> Option<Self> {
        let mut hold = (H::EMPTY_CHECKSUM, H::INITIAL_STATE);
        let mut i = 0;
        let mut ring = Vec::with_capacity(hasher.width());

        while let Some(p) = it.next() {
            let byte = *p.borrow();

            if i == hasher.width() {
                break;
            }

            let (_, state) = hold;
            hold = hasher.process_byte(state, 0, byte);
            ring.push(byte);
            i += 1;
        }
        let (sum, state) = hold;

        (i == hasher.width()).and_some(Self {
            hasher,
            next: Some(sum),
            state,
            begin: 0,
            ring,
            bytes: it,
        })
    }

    fn feed(&mut self, byte: u8) -> H::Checksum {
        // mild hack
        let mut dummy = H::INITIAL_STATE;
        core::mem::swap(&mut dummy, &mut self.state);
        let prev_state = dummy;

        let (sum, new_state) = self
            .hasher
            .process_byte(prev_state, self.ring[self.begin], byte);
        self.state = new_state;
        self.ring[self.begin] = byte;
        self.begin += 1;
        if self.begin == self.hasher.width() {
            self.begin = 0;
        }

        sum
    }
}

#[cfg(feature = "std")]
impl<H, I, X> Iterator for Rolling<H, I>
where
    H: Hasher,
    I: Iterator<Item = X>,
    X: Borrow<u8>,
{
    type Item = H::Checksum;

    fn next(&mut self) -> Option<Self::Item> {
        let mut new_next = self.bytes.next().map(|p| self.feed(*p.borrow()));
        core::mem::swap(&mut self.next, &mut new_next);
        let old_next = new_next;

        old_next
    }
}

pub(crate) mod util {
    pub trait Switch {
        fn and_some<T>(&self, x: T) -> Option<T>;
    }

    impl Switch for bool {
        fn and_some<T>(&self, x: T) -> Option<T> {
            if *self {
                Some(x)
            } else {
                None
            }
        }
    }
}

pub mod rrs;
