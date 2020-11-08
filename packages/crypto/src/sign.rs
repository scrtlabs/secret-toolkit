use secp256k1;

pub fn pubkey(priv_key: &[u8; 32]) -> secp256k1::PublicKey {
    let pk = secp256k1::SecretKey::parse(priv_key).unwrap();
    secp256k1::PublicKey::from_secret_key(&pk)
}

pub fn sign(priv_key: &[u8; 32], data &[u8; 32]) -> secp256k1::Signature {
    let pk = secp256k1::SecretKey::parse(priv_key).unwrap();
    let msg = secp256k1::Message::parse(data);
    let sig = secp256k1::sign(&msg, &pk);

    sig.0
}
