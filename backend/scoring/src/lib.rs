#![doc = include_str!("../README.md")]

mod members;
mod model;
mod scoring;
mod stats;

pub use members::{blended_pace, current_members, daily_pace, MIN_DAYS_FOR_PACE};
pub use model::{tier_label, Club, JoinStyle, MemberSnapshot, CLUB_CAP};
pub use scoring::{
    blended_median, club_balance, score_club, score_clubs, Balance, ScoredClub, ScoringError,
    ScoringOptions, DEFAULT_BALANCE_WEIGHT, MIN_RANKED_MEMBERS, PASSENGER_FLOOR,
    PASSENGER_SHARE_OF_MEDIAN,
};
