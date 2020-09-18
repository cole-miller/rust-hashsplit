use core::convert::TryInto;
use core::num::{NonZeroU32, NonZeroUsize};

pub type Checksum = u32;
pub type State = (u32, u32);

pub enum Style {
    Rrs0,
    Rrs1,
}

impl Style {
    fn checksum_from_state(&self, state: State) -> Checksum {
        let (a, b) = state;

        match self {
            Self::Rrs0 => a + (b << 16),
            Self::Rrs1 => b + (a << 16),
        }
    }
}

pub struct Hasher {
    modulus: u32,
    offset: u32,
    style: Style,
    width: NonZeroU32,
}

impl Hasher {
    pub fn new(modulus: u32, offset: u32, style: Style, width: NonZeroU32) -> Self {
        Self {
            modulus,
            offset,
            style,
            width,
        }
    }
}

impl crate::Hasher for Hasher {
    type Checksum = Checksum;

    type State = State;

    const EMPTY_CHECKSUM: Checksum = 0;

    const INITIAL_STATE: State = (0, 0);

    fn width(&self) -> NonZeroUsize {
        unsafe {
            NonZeroUsize::new_unchecked(
                self.width
                    .get()
                    .try_into()
                    .expect("lossy conversion from `u32` to `usize` may violate invariants"),
            )
        }
    }

    fn process_byte(&self, state: State, old_byte: u8, new_byte: u8) -> (Checksum, State) {
        let (a, b) = state;
        let a_new = (a - old_byte as u32 + new_byte as u32) % self.modulus;
        let b_new = (b - self.width.get() * (old_byte as u32 + self.offset) + a_new) % self.modulus;
        let new_state = (a_new, b_new);
        let sum = self.style.checksum_from_state(new_state);

        (sum, new_state)
    }
}

#[deprecated(note = "Use `rrs1` instead")]
pub fn rrs0(width: NonZeroU32) -> Hasher {
    Hasher::new(1 << 16, 31, Style::Rrs0, width)
}

pub fn rrs1(width: NonZeroU32) -> Hasher {
    Hasher::new(1 << 16, 31, Style::Rrs1, width)
}

#[cfg(feature = "std")]
pub type Rolling<I> = crate::Rolling<Hasher, I>;
