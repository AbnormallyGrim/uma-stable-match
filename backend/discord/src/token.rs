//! Signed, expiring values for the sign-in `state` and the session, verified by their signature.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::config::SigningKey;

#[derive(Serialize, Deserialize)]
struct Payload<T> {
    /// Expiry, in Unix seconds.
    e: u64,
    v: T,
}

/// Unix seconds, or 0 before 1970.
pub(crate) fn seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// `value` as `<payload>.<signature>`, valid until `expires` (Unix seconds). `purpose` is signed
/// too, which ties each value to one use, apart from the human check's tokens.
pub(crate) fn seal<T: Serialize>(
    key: &SigningKey,
    purpose: &str,
    value: &T,
    expires: u64,
) -> String {
    let json = serde_json::to_vec(&Payload {
        e: expires,
        v: value,
    })
    .expect("values serialize");
    let payload = URL_SAFE_NO_PAD.encode(json);
    let tag = URL_SAFE_NO_PAD.encode(mac(key, purpose, &payload).finalize().into_bytes());
    format!("{payload}.{tag}")
}

/// The value in `token`, if `key` signed it for `purpose` and it hasn't expired at `now`.
pub(crate) fn open<T: DeserializeOwned>(
    key: &SigningKey,
    purpose: &str,
    token: &str,
    now: SystemTime,
) -> Option<T> {
    open_expiring(key, purpose, token, now).map(|(value, _)| value)
}

/// Like [`open`], with the expiry (Unix seconds) too.
pub(crate) fn open_expiring<T: DeserializeOwned>(
    key: &SigningKey,
    purpose: &str,
    token: &str,
    now: SystemTime,
) -> Option<(T, u64)> {
    let (payload, tag) = token.split_once('.')?;
    let tag = URL_SAFE_NO_PAD.decode(tag).ok()?;
    mac(key, purpose, payload).verify_slice(&tag).ok()?;
    let payload: Payload<T> =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()?;
    (payload.e > seconds(now)).then_some((payload.v, payload.e))
}

/// A random value for a nonce, as URL-safe text.
pub(crate) fn random() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the operating system's random number generator works");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn mac(key: &SigningKey, purpose: &str, payload: &str) -> Hmac<Sha256> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(format!("discord.{purpose}.v1.{payload}").as_bytes());
    mac
}

#[cfg(test)]
mod tests;
