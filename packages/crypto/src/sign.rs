use secp256k1;

pub fn pubkey(priv_key: &[u8; 32]) -> secp256k1::PublicKey {
    let pk = secp256k1::SecretKey::parse(priv_key).unwrap();
    secp256k1::PublicKey::from_secret_key(&pk)
}

pub fn sign(priv_key: &[u8; 32], data: &[u8; 32]) -> secp256k1::Signature {
    let pk = secp256k1::SecretKey::parse(priv_key).unwrap();
    let msg = secp256k1::Message::parse(data);
    let sig = secp256k1::sign(&msg, &pk);

    sig.0
}

pub fn verify(data: &[u8; 32], signature: secp256k1::Signature, pub_key: &[u8; 65]) -> bool {
    let msg = secp256k1::Message::parse(data);
    let pk = secp256k1::PublicKey::parse(pub_key).unwrap();
    secp256k1::verify(&msg, &signature, &pk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha_256;
    use secp256k1_test::{rand::thread_rng, Secp256k1};

    #[test]
    fn test_sign() {
        let s = Secp256k1::new();
        let (secp_privkey, secp_pubkey) = s.generate_keypair(&mut thread_rng());

        let mut privkey_a = [0u8; 32];
        for i in 0..32 {
            privkey_a[i] = secp_privkey[i];
        }
 
        let data = sha_256(b"test");
        let signature = sign(&privkey_a, &data);

        assert_eq!(verify(&data, signature, &secp_pubkey.serialize_uncompressed()), true);
    }
}
