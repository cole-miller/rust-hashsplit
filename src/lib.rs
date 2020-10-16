#![no_std]
#![feature(min_const_generics)]

#[cfg(feature = "alloc")]
extern crate alloc;

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
