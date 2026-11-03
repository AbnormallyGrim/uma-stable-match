//! Lets Discord's opt-in find clubs through the club details cache.

use uma_stable_match_discord::{ClubLookup, ClubSummary, LookupError};
use uma_stable_match_umamoe_client::Priority;

use crate::details::ClubDetails;
use crate::source::SourceError;

/// A club lookup for Discord that reads `details`, fetching a club's details if they aren't cached.
/// A club the source doesn't know is `None`; any other failure is an error.
pub fn club_lookup(details: ClubDetails) -> ClubLookup {
    ClubLookup::new(move |club_id| {
        let details = details.clone();
        async move {
            match details.get(club_id, Priority::Interactive).await {
                Ok(club) => Ok(Some(ClubSummary {
                    description: club.comment.clone(),
                })),
                Err(error) if matches!(*error, SourceError::NotFound) => Ok(None),
                Err(error) => {
                    tracing::warn!(origin = error.origin(), "could not load a club for Discord");
                    Err(LookupError)
                }
            }
        }
    })
}
