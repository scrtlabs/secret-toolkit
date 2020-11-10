pub struct PrivateKey {
    inner: secp256k1::SecretKey,
}

pub struct PublicKey {
    inner: secp256k1::PublicKey,
}

impl PrivateKey {
    pub fn parse(raw: &[u8; 32]) -> Result<Self, secp256k1::Error> {
        match secp256k1::SecretKey::parse(raw) {
            Ok(key) => Ok(PrivateKey { inner: key }),
            Err(err) => Err(err),
        }
    }

    pub fn pubkey(&self) -> PublicKey {
        PublicKey {
            inner: secp256k1::PublicKey::from_secret_key(&self.inner),
        }
    }

    pub fn sign(&self, data: &[u8; 32]) -> secp256k1::Signature {
        let msg = secp256k1::Message::parse(data);
        let sig = secp256k1::sign(&msg, &self.inner);

        sig.0
    }
}

impl PublicKey {
    pub fn verify(&self, data: &[u8; 32], signature: secp256k1::Signature) -> bool {
        let msg = secp256k1::Message::parse(data);
        secp256k1::verify(&msg, &signature, &self.inner)
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

        let mut privkey = [0u8; 32];
        // privkey.copy_from_slice(secp_privkey.index());
        for i in 0..32 {
            privkey[i] = secp_privkey[i];
        }

        let new_pubkey = PrivateKey::parse(&privkey).unwrap().pubkey();

        // NOTE: These are two different type definition,
        // the new_pubkey is libsecp256k1::PublicKey
        // the secp_pubkey is secp256k1_test::PublicKey
        assert_eq!(
            new_pubkey.inner.serialize(),
            secp_pubkey.serialize_uncompressed()
        );
    }

    #[test]
    fn test_sign() {
        let s = Secp256k1::new();
        let (secp_privkey, _) = s.generate_keypair(&mut thread_rng());

        let mut privkey = [0u8; 32];
        for i in 0..32 {
            privkey[i] = secp_privkey[i];
        }

        let data = sha_256(b"test");
        let pk = PrivateKey::parse(&privkey).unwrap();
        let signature = pk.sign(&data);

        let pubkey = pk.pubkey();
        assert!(pubkey.verify(&data, signature));
    }
}
