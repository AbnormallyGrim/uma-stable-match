//! Club scoring for Uma Stable Match.
//!
//! Ranks Umamusume: Pretty Derby clubs by how well they suit a trainer. The
//! crate has no network or storage dependencies: club and member data goes in,
//! scores come out.
//!
//! # Overview
//!
//! - [`daily_pace`] works out a trainer's average daily fan gain.
//! - [`current_members`] decides who is still in a club.
//! - [`score_clubs`] ranks clubs by fit, optionally adjusted by
//!   [balance](club_balance). Lower scores are better.
//!
//! # Example
//!
//! ```
//! use uma_stable_match_scoring::{daily_pace, score_clubs, Club, ScoringOptions};
//!
//! let my_fans = [150_000_000, 157_500_000, 165_000_000];
//! let pace = daily_pace(&my_fans).expect("at least three days of data");
//!
//! let clubs: Vec<Club> = Vec::new(); // load clubs and member snapshots here
//! let options = ScoringOptions { balance_weight: Some(0.5), ..Default::default() };
//!
//! for club in score_clubs(pace, &clubs, &options)? {
//!     println!("{}: place {} of {}, score {:.3}", club.club.name, club.your_place, club.club_size, club.score);
//! }
//! # Ok::<(), uma_stable_match_scoring::ScoringError>(())
//! ```
//!
//! # Features
//!
//! - `serde`: derives `Serialize` and `Deserialize` for the input types,
//!   [`ScoringOptions`] and [`Balance`].

mod members;
mod model;
mod scoring;
mod stats;

pub use members::{current_members, daily_pace, MIN_DAYS_FOR_PACE};
pub use model::{tier_label, Club, JoinStyle, MemberSnapshot, CLUB_CAP};
pub use scoring::{
    club_balance, score_club, score_clubs, Balance, ScoredClub, ScoringError, ScoringOptions,
    DEFAULT_BALANCE_WEIGHT, MIN_RANKED_MEMBERS, PASSENGER_FLOOR, PASSENGER_SHARE_OF_MEDIAN,
};
