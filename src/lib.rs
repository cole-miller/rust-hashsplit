#![no_std]
#![deny(unsafe_code)]
#![feature(min_const_generics)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[allow(unused)]
use crate::util::*;

pub const WINDOW_SIZE: usize = 64;

pub trait Leveled {
    fn level(self) -> u32;
}

/// ```
/// use hashsplit::Leveled;
///
/// assert_eq!(false.level(), 0);
/// assert_eq!(true.level(), 1);
/// ```
impl Leveled for bool {
    fn level(self) -> u32 {
        self as u32
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

pub trait Hasher {
    type Checksum: Default;

    type State;

    const INITIAL_STATE: Self::State;

    fn process_byte(
        &self,
        state: Self::State,
        old_byte: u8,
        new_byte: u8,
    ) -> (Self::Checksum, Self::State);

    fn process_sequence<I: IntoIterator<Item = (u8, u8)>>(
        &self,
        state: Self::State,
        bytes: I,
    ) -> (Self::Checksum, Self::State) {
        bytes.into_iter().fold(
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
#[cfg(feature = "alloc")]
pub mod chunk;
pub mod config;
pub mod iter;
pub mod thin;

pub use config::Config;
