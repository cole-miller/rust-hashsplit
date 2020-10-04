/*!
The RRS family of hash functions, used by rsync and others.
*/

use crate::Hasher;

use core::num::NonZeroUsize;

pub type Checksum = u32;

pub type State = (u32, u32);

/// Trivial struct describing a hash function in the RRS family. The run-time representation of
/// this type is empty, so it is useful only as a generic parameter.
#[derive(Default)]
pub struct Rrs<const MODULUS: u32, const OFFSET: u32>;

impl<const MODULUS: u32, const OFFSET: u32> Hasher for Rrs<MODULUS, OFFSET> {
    type Checksum = Checksum;

    type State = State;

    const INITIAL_STATE: State = (0, 0);

    fn process_byte(
        &self,
        state: State,
        width: NonZeroUsize,
        old_byte: u8,
        new_byte: u8,
    ) -> (Checksum, State) {
        process_byte_freestanding::<MODULUS, OFFSET>(state, width, old_byte, new_byte)
    }
}

/// The `Hasher::process_byte` method on `Rrs`, provided here in standalone form so that it can be
/// used as a `const` function.
pub const fn process_byte_freestanding<const MODULUS: u32, const OFFSET: u32>(
    state: State,
    width: NonZeroUsize,
    old_byte: u8,
    new_byte: u8,
) -> (Checksum, State) {
    let width = width.get() as u32;

    let (a, b) = state;
    let a_new = (a - old_byte as u32 + new_byte as u32) % MODULUS;
    let b_new = (b - width * (old_byte as u32 + OFFSET) + a_new) % MODULUS;
    let new_state = (a_new, b_new);
    let sum = b_new + (a_new << 16);

    (sum, new_state)
}

pub type Rrs1 = Rrs<25_536, 31>;
