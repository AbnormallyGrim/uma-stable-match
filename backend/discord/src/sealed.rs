//! Encrypts the Discord IDs kept in saved applications, so reading Redis doesn't reveal them.

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::config::KeySecret;

const VERSION: &str = "v1.";
const NONCE_LEN: usize = 12;

/// Encrypts and decrypts short values with a key derived from the [`KeySecret`].
#[derive(Clone)]
pub(crate) struct Sealer(Aes256Gcm);

impl Sealer {
    pub(crate) fn new(secret: &KeySecret) -> Self {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(b"discord.applications.encryption.v1");
        let key = mac.finalize().into_bytes();
        Self(Aes256Gcm::new_from_slice(&key).expect("a SHA-256 output is a valid AES-256 key"))
    }

    /// `plain` encrypted with a fresh nonce, tied to `context` so it can't be moved to another record.
    pub(crate) fn seal(&self, plain: &str, context: &str) -> String {
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce).expect("the operating system's random number generator works");
        let encrypted = self
            .0
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: plain.as_bytes(),
                    aad: context.as_bytes(),
                },
            )
            .expect("AES-GCM encrypts short values");
        let mut bytes = nonce.to_vec();
        bytes.extend(encrypted);
        format!("{VERSION}{}", URL_SAFE_NO_PAD.encode(bytes))
    }

    /// The value sealed with this `context`, or `None` if it was altered, sealed with another key
    /// or context, or isn't sealed at all.
    pub(crate) fn open(&self, sealed: &str, context: &str) -> Option<String> {
        let bytes = URL_SAFE_NO_PAD.decode(sealed.strip_prefix(VERSION)?).ok()?;
        let (nonce, encrypted) = bytes.split_at_checked(NONCE_LEN)?;
        let nonce = Nonce::try_from(nonce).ok()?;
        let plain = self
            .0
            .decrypt(
                &nonce,
                Payload {
                    msg: encrypted,
                    aad: context.as_bytes(),
                },
            )
            .ok()?;
        String::from_utf8(plain).ok()
    }
}

#[cfg(test)]
mod tests;
