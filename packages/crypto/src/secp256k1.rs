use rand_core::RngCore;
pub use secp256k1::util::{MESSAGE_SIZE, SIGNATURE_SIZE};

use cosmwasm_std::StdError;

pub const PRIVATE_KEY_SIZE: usize = secp256k1::util::SECRET_KEY_SIZE;
pub const PUBLIC_KEY_SIZE: usize = secp256k1::util::FULL_PUBLIC_KEY_SIZE;
pub const COMPRESSED_PUBLIC_KEY_SIZE: usize = secp256k1::util::COMPRESSED_PUBLIC_KEY_SIZE;

pub struct PrivateKey {
    inner: secp256k1::SecretKey,
}

pub struct PublicKey {
    inner: secp256k1::PublicKey,
}

pub struct Signature {
    inner: secp256k1::Signature,
}

impl PrivateKey {
    pub fn parse(raw: &[u8; PRIVATE_KEY_SIZE]) -> Result<Self, StdError> {
        secp256k1::SecretKey::parse(raw)
            .map(|key| PrivateKey { inner: key })
            .map_err(|err| StdError::generic_err(format!("Error parsing PrivateKey: {}", err)))
    }

    pub fn generate<R: RngCore>(rng: &mut R) -> PrivateKey {
        let inner = secp256k1::SecretKey::random(rng);
        PrivateKey { inner }
    }

    pub fn serialize(&self) -> [u8; PRIVATE_KEY_SIZE] {
        self.inner.serialize()
    }

    pub fn pubkey(&self) -> PublicKey {
        PublicKey {
            inner: secp256k1::PublicKey::from_secret_key(&self.inner),
        }
    }

    pub fn sign(&self, data: &[u8; MESSAGE_SIZE]) -> Signature {
        let msg = secp256k1::Message::parse(data);
        let sig = secp256k1::sign(&msg, &self.inner);

        Signature { inner: sig.0 }
    }
}

impl PublicKey {
    pub fn parse(p: &[u8]) -> Result<PublicKey, StdError> {
        secp256k1::PublicKey::parse_slice(p, None)
            .map(|key| PublicKey { inner: key })
            .map_err(|err| StdError::generic_err(format!("Error parsing PublicKey: {}", err)))
    }

    pub fn serialize(&self) -> [u8; PUBLIC_KEY_SIZE] {
        self.inner.serialize()
    }

    pub fn serialize_compressed(&self) -> [u8; COMPRESSED_PUBLIC_KEY_SIZE] {
        self.inner.serialize_compressed()
    }

    pub fn verify(&self, data: &[u8; MESSAGE_SIZE], signature: Signature) -> bool {
        let msg = secp256k1::Message::parse(data);
        secp256k1::verify(&msg, &signature.inner, &self.inner)
    }
}

impl Signature {
    pub fn parse(p: &[u8; SIGNATURE_SIZE]) -> Signature {
        Signature {
            inner: secp256k1::Signature::parse(p),
        }
    }

    pub fn parse_slice(p: &[u8]) -> Result<Signature, StdError> {
        secp256k1::Signature::parse_slice(p)
            .map(|sig| Signature { inner: sig })
            .map_err(|err| StdError::generic_err(format!("Error parsing Signature: {}", err)))
    }

    pub fn serialize(&self) -> [u8; SIGNATURE_SIZE] {
        self.inner.serialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sha_256, Prng};
    use secp256k1_test::{rand::thread_rng, Secp256k1};

    #[test]
    fn test_pubkey() {
        let s = Secp256k1::new();
        let (secp_privkey, secp_pubkey) = s.generate_keypair(&mut thread_rng());

        let mut privkey = [0u8; PRIVATE_KEY_SIZE];
        privkey.copy_from_slice(&secp_privkey[..]);

        let new_pubkey = PrivateKey::parse(&privkey).unwrap().pubkey();

        assert_eq!(
            new_pubkey.inner.serialize(),
            secp_pubkey.serialize_uncompressed()
        );
    }

    #[test]
    fn test_sign() {
        let s = Secp256k1::new();
        let (secp_privkey, _) = s.generate_keypair(&mut thread_rng());

        let mut privkey = [0u8; PRIVATE_KEY_SIZE];
        privkey.copy_from_slice(&secp_privkey[..]);

        let data = sha_256(b"test");
        let pk = PrivateKey::parse(&privkey).unwrap();
        let signature = pk.sign(&data);

        let pubkey = pk.pubkey();
        assert!(pubkey.verify(&data, signature));
    }

    #[test]
    fn test_gen_privkey_prng() {
        let mut prng = Prng::new(b"seed", b"totaly_random_entropy");
        let deterministic_privk_bytes = [
            231, 69, 203, 68, 12, 68, 1, 65, 112, 124, 211, 125, 255, 27, 232, 141, 172, 221, 75,
            35, 83, 175, 8, 34, 15, 213, 110, 92, 215, 144, 20, 210,
        ];
        let privk = PrivateKey::generate(&mut prng);
        assert_eq!(deterministic_privk_bytes, privk.serialize());
    }
}
