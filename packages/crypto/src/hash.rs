use sha2::{Digest, Sha256};

pub const SHA256_HASH_SIZE: usize = 32;

pub fn sha_256(data: &[u8]) -> [u8; SHA256_HASH_SIZE] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_slice());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha_256() {
        let r = sha_256(b"test");
        let r_expected: [u8; SHA256_HASH_SIZE] = [
            159, 134, 208, 129, 136, 76, 125, 101, 154, 47, 234, 160, 197, 90, 208, 21, 163, 191,
            79, 27, 43, 11, 130, 44, 209, 93, 108, 21, 176, 240, 10, 8,
        ];
        assert_eq!(r, r_expected);

        let r = sha_256(b"random_string_123");
        let r_expected: [u8; SHA256_HASH_SIZE] = [
            167, 75, 46, 161, 27, 233, 254, 146, 245, 218, 2, 19, 171, 56, 78, 166, 42, 211, 88, 7,
            205, 191, 2, 6, 226, 158, 43, 144, 8, 149, 170, 164,
        ];
        assert_eq!(r, r_expected);
    }
}
