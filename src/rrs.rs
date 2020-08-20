extern crate core;

use core::prelude::v1::*;

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
    width: u32,
}

impl Hasher {
    pub fn new(modulus: u32, offset: u32, style: Style, width: u32) -> Self {
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
        self.width as usize
    }

    fn empty_checksum() -> Checksum {
        0
    }

    fn initial_state() -> State {
        (0, 0)
    }

    fn process_byte(&self, state: State, old_byte: u8, new_byte: u8) -> (Checksum, State) {
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

    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "sse2",
        target_feature = "ssse3",
    ))]
    unsafe fn process_chunk128(
        &self,
        state: State,
        old_data: &[u8; 16],
        new_data: &[u8; 16],
    ) -> (Checksum, State) {
        #[cfg(target_arch = "x86")]
        use core::arch::x86::*;
        #[cfg(target_arch = "x86_64")]
        use core::arch::x86_64::*;

        let (a_prev, _) = state;

        let (old_loaded, new_loaded) = (
        // note: unaligned loads
            _mm_loadu_si128(old_data.as_ptr().cast()),
        //  ^ oooooooooooooooo
            _mm_loadu_si128(new_data.as_ptr().cast()),
        //  ^ nnnnnnnnnnnnnnnn
        );

        let zeroed = _mm_setzero_si128();
        let (old_summed, new_summed) = (
            _mm_sad_epu8(old_loaded, zeroed),
        //  ^ oo000000_oo000000
            _mm_sad_epu8(new_loaded, zeroed),
        //  ^ nn000000_nn000000
        );
        let both_summed = _mm_hadd_epi32(old_summed, new_summed);
        //  ^ oo00_oo00_nn00_nn00
        let both_summed = _mm_hadd_epi32(both_summed, zeroed);
        //  ^ oo00_nn00_0000_0000
        let old_summed = _mm_extract_epi32(both_summed, 0) as u32;
        let a_new = (a_prev - old_summed + _mm_extract_epi32(both_summed, 0) as u32) % self.modulus;

        let (old_factors, new_factors) = (
            _mm_set_epi8(
                17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ),
            _mm_set_epi8(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16),
        );
        let (old_smushed, new_smushed) = (
            _mm_maddubs_epi16(old_loaded, old_factors),
        //  ^ oo_oo_oo_oo_oo_oo_oo_oo
            _mm_maddubs_epi16(new_loaded, new_factors),
        //  ^ nn_nn_nn_nn_nn_nn_nn_nn
        );
        // each of the 16-bit unsigned integers packed in {old,new}_smushed is at most 0x20 *
        // 0xFF = 0x3F_FC, so we have at least six leading zero bits with which to avoid overflow,
        // justifying the following sums
        let both_stacked = _mm_hadd_epi16(old_smushed, new_smushed);
        //  ^ oo_oo_oo_oo_nn_nn_nn_nn
        let both_stacked = _mm_hadd_epi16(both_stacked, zeroed);
        //  ^ oo_oo_nn_nn_00_00_00_00
        let both_stacked = _mm_hadd_epi16(both_stacked, zeroed);
        //  ^ oo_nn_00_00_00_00_00_00
        let b_new = (0u32 - 16u32 * self.width * self.offset - self.width * old_summed
            + 16u32 * a_prev
            - _mm_extract_epi16(both_stacked, 0) as u32
            + _mm_extract_epi16(both_stacked, 1) as u32)
            % self.modulus;

        let sum = match self.style {
            Style::Rrs0 => a_new + (b_new << 16),
            Style::Rrs1 => b_new + (a_new << 16),
        };

        (sum, (a_new, b_new))
    }
}

#[deprecated(note = "Use `rrs1` instead.")]
pub fn rrs0(width: u32) -> Hasher {
    Hasher::new(1 << 16, 31, Style::Rrs0, width)
}

pub fn rrs1(width: u32) -> Hasher {
    Hasher::new(1 << 16, 31, Style::Rrs1, width)
}
