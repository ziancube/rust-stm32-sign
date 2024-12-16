use alloc::vec::Vec;

use ecdsa::{signature::Signer, Signature};
use k256::ecdsa::SigningKey;
use rand_core::{CryptoRng, CryptoRngCore};
use rsa::traits::SignatureScheme;
use sha2::Digest;
use rsa::RsaPrivateKey;
use rsa::pkcs1v15;
use defmt::*;

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

pub fn rsa_key(rng: &mut impl CryptoRngCore) -> RsaPrivateKey {
    let key = RsaPrivateKey::new(rng, 2048).unwrap();
    key
}

pub fn rsa_sign(message: &[u8], key: &RsaPrivateKey) -> Vec<u8> {
    let digest = sha2::Sha256::digest(message);
    let sig = key.sign(pkcs1v15::Pkcs1v15Sign::new::<sha2::Sha256>(), &digest).unwrap();
    sig
}