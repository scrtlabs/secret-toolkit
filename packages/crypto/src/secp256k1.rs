pub use secp256k1::constants::{MESSAGE_SIZE, COMPACT_SIGNATURE_SIZE as SIGNATURE_SIZE};
use secp256k1::{ecdsa::Signature as SecpSignature};

use cosmwasm_std::StdError;

pub const PRIVATE_KEY_SIZE: usize = secp256k1::constants::SECRET_KEY_SIZE;
pub const PUBLIC_KEY_SIZE: usize = secp256k1::constants::UNCOMPRESSED_PUBLIC_KEY_SIZE;
pub const COMPRESSED_PUBLIC_KEY_SIZE: usize = secp256k1::constants::PUBLIC_KEY_SIZE;

pub struct PrivateKey {
    inner: secp256k1::SecretKey,
}

pub struct PublicKey {
    inner: secp256k1::PublicKey,
}

pub struct Signature {
    inner: SecpSignature,
}

impl PrivateKey {
    pub fn parse(raw: &[u8; PRIVATE_KEY_SIZE]) -> Result<Self, StdError> {
        secp256k1::SecretKey::from_slice(raw)
            .map(|key| PrivateKey { inner: key })
            .map_err(|err| StdError::generic_err(format!("Error parsing PrivateKey: {}", err)))
    }

    pub fn serialize(&self) -> [u8; PRIVATE_KEY_SIZE] {
        self.inner.serialize_secret()
    }

    pub fn pubkey(&self) -> PublicKey {
        let secp = secp256k1::Secp256k1::new();
        PublicKey {
            inner: secp256k1::PublicKey::from_secret_key(&secp, &self.inner),
        }
    }

    pub fn sign(&self, data: &[u8; MESSAGE_SIZE]) -> Signature {
        // will never fail since `from_slice` only fails when a non 32 byte input is given
        let msg = secp256k1::Message::from_slice(data).unwrap();

        let sign = secp256k1::Secp256k1::signing_only();
        let sig = sign.sign_ecdsa(&msg, &self.inner);

        Signature { inner: sig }
    }
}

impl PublicKey {
    pub fn parse(p: &[u8]) -> Result<PublicKey, StdError> {
        secp256k1::PublicKey::from_slice(p)
            .map(|key| PublicKey { inner: key })
            .map_err(|err| StdError::generic_err(format!("Error parsing PublicKey: {}", err)))
    }

    pub fn serialize(&self) -> [u8; PUBLIC_KEY_SIZE] {
        self.inner.serialize_uncompressed()
    }

    pub fn serialize_compressed(&self) -> [u8; COMPRESSED_PUBLIC_KEY_SIZE] {
        self.inner.serialize()
    }

    pub fn verify(&self, data: &[u8; MESSAGE_SIZE], signature: Signature) -> bool {
        // will never fail since `from_slice` only fails when a non 32 byte input is given
        let msg = secp256k1::Message::from_slice(data).unwrap();

        let verifier = secp256k1::Secp256k1::verification_only();

        verifier.verify_ecdsa(&msg, &signature.inner, &self.inner).is_ok()
    }
}

impl Signature {
    pub fn parse(p: &[u8; SIGNATURE_SIZE]) -> Result<Signature, StdError> {
        SecpSignature::from_compact(p)
            .map(|sig| Signature { inner: sig })
            .map_err(|err| StdError::generic_err(format!("Error parsing Signature: {}", err)))
    }

    pub fn parse_slice(p: &[u8]) -> Result<Signature, StdError> {
        SecpSignature::from_compact(p)
            .map(|sig| Signature { inner: sig })
            .map_err(|err| StdError::generic_err(format!("Error parsing Signature: {}", err)))
    }

    pub fn serialize(&self) -> [u8; SIGNATURE_SIZE] {
        self.inner.serialize_compact()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha_256;
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
}
