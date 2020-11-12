use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};
use sha2::{Digest, Sha256};

pub struct Prng {
    rng: ChaChaRng,
}

impl Prng {
    pub fn new(seed: &[u8], entropy: &[u8]) -> Self {
        let mut hasher = Sha256::new();

        // write input message
        hasher.update(&seed);
        hasher.update(&entropy);
        let hash = hasher.finalize();

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(hash.as_slice());

        let rng = ChaChaRng::from_seed(hash_bytes);

        Self { rng }
    }

    pub fn rand_bytes(&mut self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        self.rng.fill_bytes(&mut bytes);

        bytes
    }

    pub fn set_word_pos(&mut self, count: u32) {
        self.rng.set_word_pos(count.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This test checks that the rng is stateful and generates
    /// different random bytes every time it is called.
    #[test]
    fn test_rng() {
        let mut rng = Prng::new(b"foo", b"bar!");
        let r1: [u8; 32] = [
            155, 11, 21, 97, 252, 65, 160, 190, 100, 126, 85, 251, 47, 73, 160, 49, 216, 182, 93,
            30, 185, 67, 166, 22, 34, 10, 213, 112, 21, 136, 49, 214,
        ];
        let r2: [u8; 32] = [
            46, 135, 19, 242, 111, 125, 59, 215, 114, 130, 122, 155, 202, 23, 36, 118, 83, 11, 6,
            180, 97, 165, 218, 136, 134, 243, 191, 191, 149, 178, 7, 149,
        ];
        let r3: [u8; 32] = [
            9, 2, 131, 50, 199, 170, 6, 68, 168, 28, 242, 182, 35, 114, 15, 163, 65, 139, 101, 221,
            207, 147, 119, 110, 81, 195, 6, 134, 14, 253, 245, 244,
        ];
        let r4: [u8; 32] = [
            68, 196, 114, 205, 225, 64, 201, 179, 18, 77, 216, 197, 211, 13, 21, 196, 11, 102, 106,
            195, 138, 250, 29, 185, 51, 38, 183, 0, 5, 169, 65, 190,
        ];
        assert_eq!(r1, rng.rand_bytes());
        assert_eq!(r2, rng.rand_bytes());
        assert_eq!(r3, rng.rand_bytes());
        assert_eq!(r4, rng.rand_bytes());
    }

    #[test]
    fn test_rand_bytes_counter() {
        let mut rng = Prng::new(b"foo", b"bar");

        let r1: [u8; 32] = [
            114, 227, 179, 76, 120, 34, 236, 42, 204, 27, 153, 74, 44, 29, 158, 162, 180, 202, 165,
            46, 155, 90, 178, 252, 127, 80, 162, 79, 3, 146, 153, 88,
        ];

        rng.set_word_pos(8);
        assert_eq!(r1, rng.rand_bytes());
        rng.set_word_pos(8);
        assert_eq!(r1, rng.rand_bytes());
        rng.set_word_pos(9);
        assert_ne!(r1, rng.rand_bytes());
    }
}
