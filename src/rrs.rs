#[allow(dead_code)]

pub struct Hasher {
    modulus: u32,
    offset: u32,
    width: usize,
}

pub type Checksum = u32;
pub type State = (u32, u32);

impl Hasher {
    pub fn new(modulus: u32, offset: u32, width: usize) -> Self {
        Self {
            modulus,
            offset,
            width,
        }
    }

    pub fn initial_state() -> State {
        (0, 0)
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn process_byte(&self, state: State, old_byte: u8, new_byte: u8) -> (Checksum, State) {
        let (a, b) = state;
        let a_new = (a - old_byte as u32 + new_byte as u32) % self.modulus;
        let b_new =
            (b - self.width as u32 * (old_byte as u32 + self.offset) + a_new) % self.modulus;
        let sum = b_new + (a_new << 16);

        (sum, (a_new, b_new))
    }

    pub fn roll<I>(self, it: I) -> Rolling<I>
    where
        I: Iterator<Item = u8>,
    {
        Rolling::start(self, it)
    }
}

pub struct Rolling<I> {
    hasher: Hasher,
    next: Option<Checksum>,
    state: State,
    begin: usize,
    ring: Vec<u8>,
    bytes: I,
}

impl<I> Rolling<I>
where
    I: Iterator<Item = u8>,
{
    fn start(hasher: Hasher, mut it: I) -> Self {
        let mut hold = (0, Hasher::initial_state()); // XXX
        let mut i = 0;
        let mut ring = Vec::with_capacity(hasher.width);

        while let Some(byte) = it.next() {
            if i == hasher.width() {
                break;
            }

            let (_, state) = hold;
            hold = hasher.process_byte(state, 0, byte);
            ring.push(byte);
            i += 1;
        }
        let (sum, state) = hold;

        if i < hasher.width() {
            panic!("you didn't give me enough bytes!");
        }

        Self {
            hasher,
            next: Some(sum),
            state,
            begin: 0,
            ring,
            bytes: it,
        }
    }

    fn feed(&mut self, byte: u8) {
        let (sum, state_new) = self
            .hasher
            .process_byte(self.state, self.ring[self.begin], byte);
        self.next = Some(sum);
        self.state = state_new;
        self.ring[self.begin] = byte;
        self.begin = (self.begin + 1) % self.hasher.width();
    }
}

impl<I> Iterator for Rolling<I>
where
    I: Iterator<Item = u8>,
{
    type Item = Checksum;

    fn next(&mut self) -> Option<Self::Item> {
        let saved = self.next;

        if let Some(byte) = self.bytes.next() {
            self.feed(byte);
        } else {
            self.next = None;
        }

        saved
    }
}

pub fn rrs1(width: usize) -> Hasher {
    Hasher::new(1 << 16, 31, width)
}
