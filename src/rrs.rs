pub type Checksum = u32;
pub type State = (u32, u32);

pub enum Style {
    Rrs0,
    Rrs1,
}

pub struct Hasher {
    modulus: u32,
    offset: u32,
    style: Style,
    width: usize,
}

impl Hasher {
    pub fn new(modulus: u32, offset: u32, style: Style, width: usize) -> Self {
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

    fn width(&self) -> usize {
        self.width
    }

    fn empty_checksum() -> Checksum {
        0
    }

    fn initial_state() -> State {
        (0, 0)
    }

    fn process_byte(&self, state: &State, old_byte: u8, new_byte: u8) -> (Checksum, State) {
        let (a, b) = state;
        let a_new = (a - old_byte as u32 + new_byte as u32) % self.modulus;
        let b_new =
            (b - self.width as u32 * (old_byte as u32 + self.offset) + a_new) % self.modulus;

        let sum = match self.style {
            Style::Rrs0 => a_new + (b_new << 16),
            Style::Rrs1 => b_new + (a_new << 16),
        };

        (sum, (a_new, b_new))
    }
}

pub fn rrs0(width: usize) -> Hasher {
    Hasher::new(1 << 16, 31, Style::Rrs0, width)
}

pub fn rrs1(width: usize) -> Hasher {
    Hasher::new(1 << 16, 31, Style::Rrs1, width)
}
