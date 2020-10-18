#[allow(unused)]
use crate::util::*;
use crate::{Hasher, Named, WINDOW_SIZE};

pub type Checksum = u32;

pub type State = (u32, u32);

#[derive(Clone, Copy, Default)]
pub struct Rrs<const MODULUS: u32, const OFFSET: u32>;

impl<const MODULUS: u32, const OFFSET: u32> Hasher for Rrs<MODULUS, OFFSET> {
    type Checksum = Checksum;

    type State = State;

    const INITIAL_STATE: State = (0, 0);

    fn process_byte(&self, state: State, old_byte: u8, new_byte: u8) -> (Checksum, State) {
        process_byte_freestanding::<MODULUS, OFFSET>(state, old_byte, new_byte)
    }
}

pub const fn process_byte_freestanding<const MODULUS: u32, const OFFSET: u32>(
    state: State,
    old_byte: u8,
    new_byte: u8,
) -> (Checksum, State) {
    let (a, b) = state;
    let a_new = (a - old_byte as u32 + new_byte as u32) % MODULUS;
    let b_new = (b - WINDOW_SIZE as u32 * (old_byte as u32 + OFFSET) + a_new) % MODULUS;
    let new_state = (a_new, b_new);
    let sum = b_new + (a_new << 16);

    (sum, new_state)
}

pub type Rrs1 = Rrs<25_536, 31>;

impl Named for Rrs1 {
    const NAME: &'static str = "RRS1";
}
