//! Marks the clubs people can apply to through Discord.

use axum::Extension;
use uma_stable_match_discord::ClubStore;

use crate::recommendations::ClubInfo;

/// What `applications` says for a club people can apply to through Discord.
pub(crate) const DISCORD: &str = "discord";

/// Which clubs take applications through Discord, read from the links in Redis. Added to the app
/// as an [`Extension`] when Discord is set up.
#[derive(Debug, Clone)]
pub struct Applications {
    store: ClubStore,
}

impl Applications {
    /// Applications that read the club links in `store`.
    pub fn new(store: ClubStore) -> Self {
        Self { store }
    }
}

/// Sets `applications` to `"discord"` on the clubs that take applications through Discord: linked,
/// open, joinable (not closed or full) and with no setup problem. Without Discord, or if the links
/// can't be read, none are marked, as a missing badge is better than a wrong one.
pub(crate) async fn mark(
    applications: &Option<Extension<Applications>>,
    clubs: impl IntoIterator<Item = &mut ClubInfo>,
) {
    let Some(Extension(applications)) = applications else {
        return;
    };
    let mut joinable: Vec<&mut ClubInfo> = clubs.into_iter().filter(|c| c.joinable()).collect();
    let ids: Vec<u64> = joinable.iter().map(|club| club.circle_id).collect();
    match applications.store.takes_applications(&ids).await {
        Ok(flags) => {
            for (club, takes) in joinable.iter_mut().zip(flags) {
                club.applications = takes.then_some(DISCORD);
            }
        }
        Err(error) => {
            metrics::counter!("discord_applications_errors_total").increment(1);
            tracing::warn!(%error, "could not read which clubs take applications on Discord");
        }
    }
}
