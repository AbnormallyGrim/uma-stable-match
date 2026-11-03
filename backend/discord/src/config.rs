//! Discord application settings.

use std::fmt;
use std::time::Duration;

use ed25519_dalek::VerifyingKey;

/// Discord's REST API, version 10.
pub const DEFAULT_API_BASE: &str = "https://discord.com/api/v10";

/// The Discord application's ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationId(u64);

impl ApplicationId {
    /// The ID, or `None` if it isn't a positive whole number.
    pub fn parse(value: &str) -> Option<Self> {
        value.trim().parse().ok().filter(|id| *id > 0).map(Self)
    }

    /// The ID as a number.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// The application's public key, which signs interaction requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey(VerifyingKey);

impl PublicKey {
    /// The key from its 64 hex characters, or `None` if it isn't a valid key.
    pub fn parse(value: &str) -> Option<Self> {
        let bytes: [u8; 32] = decode_hex(value.trim())?.try_into().ok()?;
        VerifyingKey::from_bytes(&bytes).ok().map(Self)
    }

    pub(crate) fn key(&self) -> &VerifyingKey {
        &self.0
    }
}

/// The bot's token. `Debug` output hides it.
#[derive(Clone, PartialEq, Eq)]
pub struct BotToken(String);

impl BotToken {
    /// The token, or `None` if it's blank.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        (!value.is_empty()).then(|| Self(value.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for BotToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BotToken(<redacted>)")
    }
}

/// The application's OAuth2 client secret. `Debug` output hides it.
#[derive(Clone, PartialEq, Eq)]
pub struct ClientSecret(String);

impl ClientSecret {
    /// The secret, or `None` if it's blank.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        (!value.is_empty()).then(|| Self(value.to_owned()))
    }
}

impl fmt::Debug for ClientSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ClientSecret(<redacted>)")
    }
}

/// How the REST client waits out Discord's rate limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitConfig {
    /// Longest wait before a request gives up with [`Error::RateLimited`](crate::Error).
    pub max_wait: Duration,
    /// Most retries after a 429.
    pub max_retries: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_wait: Duration::from_secs(30),
            max_retries: 3,
        }
    }
}

/// Settings for the Discord application.
#[derive(Debug, Clone)]
pub struct DiscordConfig {
    /// Application ID.
    pub app_id: ApplicationId,
    /// Public key that signs interaction requests.
    pub public_key: PublicKey,
    /// Bot token for REST calls.
    pub bot_token: BotToken,
    /// OAuth2 client secret.
    pub client_secret: ClientSecret,
    /// REST API base URL, without a trailing slash.
    pub api_base: String,
    /// Rate-limit waits and retries.
    pub rate_limit: RateLimitConfig,
}

impl DiscordConfig {
    /// Settings for Discord's own API, with the default rate-limit handling.
    pub fn new(
        app_id: ApplicationId,
        public_key: PublicKey,
        bot_token: BotToken,
        client_secret: ClientSecret,
    ) -> Self {
        Self {
            app_id,
            public_key,
            bot_token,
            client_secret,
            api_base: DEFAULT_API_BASE.to_owned(),
            rate_limit: RateLimitConfig::default(),
        }
    }
}

/// Bytes from hex, or `None` if it has an odd length or a non-hex character.
pub(crate) fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    value
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let digit = |b: u8| (b as char).to_digit(16);
            Some((digit(pair[0])? * 16 + digit(pair[1])?) as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests;
