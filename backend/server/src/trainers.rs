use uma_stable_match_discord::{LookupError, TrainerLookup, TrainerSummary};

use crate::source::Source;

/// A trainer lookup for Discord that reads the trainer's profile from `source`, fresh each time.
/// A trainer the source doesn't know is `None`; any other failure is an error.
pub fn trainer_lookup(source: Source) -> TrainerLookup {
    TrainerLookup::new(move |trainer_id| {
        let source = source.clone();
        async move {
            match source.profile(trainer_id).await {
                Ok(profile) => Ok(Some(TrainerSummary {
                    name: profile.trainer.name,
                    comment: profile.trainer.comment,
                })),
                Err(error) if error.is_not_found() => Ok(None),
                Err(error) => {
                    tracing::warn!(
                        origin = error.origin(),
                        "could not load a trainer's profile for Discord"
                    );
                    Err(LookupError)
                }
            }
        }
    })
}
