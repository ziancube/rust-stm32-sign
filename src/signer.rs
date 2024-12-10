use alloc::vec::Vec;

use ecdsa::{signature::Signer, Signature};
use k256::ecdsa::SigningKey;
use rand_core::CryptoRngCore;
use sha2::Digest;

pub fn digest(message: &[u8]) -> Vec<u8> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(message);
    hasher.finalize().to_vec()
}

pub fn keygen(rng: &mut impl CryptoRngCore) -> SigningKey{
    let key = SigningKey::random(rng);
    key
}

pub fn sign(message: &[u8], key: &SigningKey) -> Vec<u8> {
    let digest = sha2::Sha256::digest(message);
    let sig: Signature<_> = key.sign(&digest);
    sig.to_vec()
}
