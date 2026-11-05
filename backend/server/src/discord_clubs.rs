//! Lets Discord's opt-in look clubs up on uma.moe.

use uma_stable_match_discord::{ClubLookup, ClubSummary, LookupError};
use uma_stable_match_umamoe_client::{CircleRequest, Priority};

use crate::source::{Source, SourceError};

/// A club lookup for Discord that reads the club from `source`, fresh each time, so a change of
/// leader or description counts at once. A club the source doesn't know is `None`; any other
/// failure is an error.
pub fn club_lookup(source: Source) -> ClubLookup {
    ClubLookup::new(move |club_id| {
        let source = source.clone();
        async move {
            let request = CircleRequest::by_circle_id(club_id);
            match source.club(&request, Priority::Interactive).await {
                Ok(found) => Ok(Some(ClubSummary {
                    name: found.circle.name,
                    description: found.circle.comment,
                    leader: found.circle.leader_viewer_id,
                })),
                Err(error) if error.is_not_found() || matches!(error, SourceError::NotFound) => {
                    Ok(None)
                }
                Err(error) => {
                    tracing::warn!(origin = error.origin(), "could not load a club for Discord");
                    Err(LookupError)
                }
            }
        }
    })
}
