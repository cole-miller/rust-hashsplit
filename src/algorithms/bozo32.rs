#[allow(unused)]
use crate::util::*;
use crate::{Hasher, Named, WINDOW_SIZE};

pub type Checksum = u32;

pub type State = u32;

const PRIME: u32 = 65_521;

// FIXME maybe `u32::wrapping_pow` will be made `const` and I can use that?
const PRIME_POW: u32 = {
    let mut i: usize = 0;
    let mut pow: u32 = 1;

    while i < WINDOW_SIZE {
        pow = pow.wrapping_mul(PRIME);
        i += 1;
    }

    pow
};

#[derive(Clone, Copy, Default)]
pub struct Bozo32;

impl Hasher for Bozo32 {
    type Checksum = Checksum;

    type State = State;

    const INITIAL_STATE: State = 0;

    fn process_byte(&self, state: State, old_byte: u8, new_byte: u8) -> (Checksum, State) {
        process_byte_freestanding(state, old_byte, new_byte)
    }
}

pub const fn process_byte_freestanding(
    state: State,
    old_byte: u8,
    new_byte: u8,
) -> (Checksum, State) {
    let sum = state * PRIME + new_byte as u32 - old_byte as u32 * PRIME_POW;

    (sum, sum)
}

impl Named for Bozo32 {
    const NAME: &'static str = "Bozo32";
}
