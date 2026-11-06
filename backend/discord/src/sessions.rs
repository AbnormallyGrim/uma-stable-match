//! Ending sessions before they expire, kept in Redis.
//!
//! - `discord:session:revoked:{token}`: a session that signed out, with `{token}` a hash of its
//!   cookie, until it would have expired.
//! - `discord:session:ended:{hash}`: when the person with that hash of their Discord ID had every
//!   session ended, in Unix seconds, for as long as a session lasts. Sessions they started before
//!   then no longer count.

use std::time::Duration;

use redis::aio::ConnectionManager;
use sha2::{Digest, Sha256};
use tokio::time::timeout;

use crate::auth::MAX_SESSION_AGE;
use crate::config::KeySecret;
use crate::store::StoreError;
use crate::trainer_store::applicant_hash;

const REVOKED_PREFIX: &str = "discord:session:revoked:";
const ENDED_PREFIX: &str = "discord:session:ended:";
const LIMIT: Duration = Duration::from_secs(5);

/// Sessions ended before they expire. Cheap to clone.
#[derive(Clone)]
pub struct SessionStore {
    conn: ConnectionManager,
    secret: KeySecret,
}

impl std::fmt::Debug for SessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionStore").finish_non_exhaustive()
    }
}

impl SessionStore {
    /// A store using this connection, hashing Discord IDs with `secret`.
    pub fn new(conn: ConnectionManager, secret: KeySecret) -> Self {
        Self { conn, secret }
    }

    /// Ends the session in `token`, which expires at `expires` (Unix seconds).
    pub async fn revoke(&self, token: &str, expires: u64, now: u64) -> Result<(), StoreError> {
        let left = expires.saturating_sub(now).max(1);
        self.run(
            redis::cmd("SET")
                .arg(revoked_key(token))
                .arg(1)
                .arg("EX")
                .arg(left)
                .query_async::<()>(&mut self.conn.clone()),
        )
        .await
    }

    /// Ends every session `user` started before `now` (Unix seconds). One started in the same
    /// second is kept, so signing in again straight away works.
    pub async fn end_all(&self, user: u64, now: u64) -> Result<(), StoreError> {
        self.run(
            redis::cmd("SET")
                .arg(self.ended_key(user))
                .arg(now)
                .arg("EX")
                .arg(MAX_SESSION_AGE.as_secs())
                .query_async::<()>(&mut self.conn.clone()),
        )
        .await
    }

    /// Whether the session in `token`, which `user` started at `started` (Unix seconds), hasn't
    /// been ended.
    pub async fn valid(&self, token: &str, user: u64, started: u64) -> Result<bool, StoreError> {
        let (revoked, ended): (Option<String>, Option<u64>) = self
            .run(
                redis::cmd("MGET")
                    .arg(revoked_key(token))
                    .arg(self.ended_key(user))
                    .query_async(&mut self.conn.clone()),
            )
            .await?;
        Ok(revoked.is_none() && ended.is_none_or(|ended| started >= ended))
    }

    fn ended_key(&self, user: u64) -> String {
        format!("{ENDED_PREFIX}{}", applicant_hash(&self.secret, user))
    }

    async fn run<T>(
        &self,
        call: impl std::future::Future<Output = redis::RedisResult<T>>,
    ) -> Result<T, StoreError> {
        timeout(LIMIT, call)
            .await
            .map_err(|_| StoreError::TimedOut)?
            .map_err(StoreError::from)
    }
}

fn revoked_key(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let hash: String = digest
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("{REVOKED_PREFIX}{hash}")
}
