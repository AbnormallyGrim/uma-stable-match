//! Starts the server with made-up fixture clubs instead of uma.moe, and a mock Discord when
//! Discord is set up, for end-to-end tests. See the server README.

mod discord;

use std::collections::HashMap;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use uma_stable_match_server::{
    init_logging, run, ClubSource, Config, LogFormat, Source, SourceError, SourceFuture,
};
use uma_stable_match_umamoe_client::{
    Circle, CircleListResponse, CircleLookup, CircleMember, CircleRequest, CircleResponse,
    GainsRankingsResponse, ListCirclesRequest, MonthlyRanking, MonthlyRankingsResponse, Priority,
    ProfileAlltime, ProfileCircle, ProfileFanHistory, ProfileResponse, ProfileTrainer,
    RankingsGainsRequest, RankingsMonthlyRequest,
};

/// Number of fixture clubs.
const CLUBS: u64 = 24;
/// Members in each fixture club, leaving room to join.
const MEMBERS: u64 = 20;
/// Days of fan data, up to the snapshot time.
const DAYS: i64 = 18;
/// When the fixture data was taken.
const SNAPSHOT: &str = "2026-03-19T15:00:00Z";
/// Rank of the top fixture club, inside the default scan window.
const FIRST_RANK: u64 = 101;
/// Member `k` of club `n` has trainer ID `TRAINER_BASE + n * 1000 + k`.
const TRAINER_BASE: u64 = 900_000_000_000;
/// Port of the mock Discord when `FIXTURE_DISCORD_PORT` isn't set.
const DEFAULT_DISCORD_PORT: u16 = 8182;
const M: i64 = 1_000_000;

/// Profile comments set through the mock Discord's control route, by trainer ID.
pub type Comments = Arc<Mutex<HashMap<u64, String>>>;

/// Made-up clubs, served from memory.
struct Fixtures {
    clubs: Vec<CircleResponse>,
    comments: Comments,
}

impl Fixtures {
    /// Club `n` gains about `n / 4` million fans a day per member, so paces from 0.25M to 6M
    /// each have clubs nearby. Every fourth club takes applications and the rest are open.
    fn new(comments: Comments) -> Self {
        Self {
            clubs: (1..=CLUBS).map(club).collect(),
            comments,
        }
    }

    fn by_member(&self, viewer_id: u64) -> Option<&CircleResponse> {
        self.clubs
            .iter()
            .find(|club| club.members.iter().any(|m| m.viewer_id == viewer_id))
    }
}

fn club(n: u64) -> CircleResponse {
    let gains: Vec<i64> = (0..MEMBERS)
        .map(|k| n as i64 * M / 4 + k as i64 * M / 50)
        .collect();
    let members = gains
        .iter()
        .zip(0..)
        .map(|(gain, k)| CircleMember {
            circle_id: n,
            viewer_id: TRAINER_BASE + n * 1000 + k,
            trainer_name: Some(format!("Fixture Trainer {n}-{k}")),
            year: Some(2026),
            month: Some(3),
            daily_fans: (1..=DAYS).map(|day| 50 * M + gain * day).collect(),
            last_updated: Some(SNAPSHOT.into()),
            ..Default::default()
        })
        .collect();
    CircleResponse {
        circle: Circle {
            circle_id: n,
            name: format!("Fixture Club {n}"),
            comment: Some(format!(
                "A made-up club. Join us on discord.gg/{}",
                discord::invite(n)
            )),
            leader_viewer_id: Some(TRAINER_BASE + n * 1000),
            leader_name: Some(format!("Fixture Trainer {n}-0")),
            member_count: Some(MEMBERS as i32),
            join_style: Some(if n.is_multiple_of(4) { 2 } else { 1 }),
            monthly_rank: Some((FIRST_RANK + CLUBS - n) as i32),
            monthly_point: Some(gains.iter().sum::<i64>() * DAYS),
            last_updated: Some(SNAPSHOT.into()),
            ..Default::default()
        },
        members,
        ..Default::default()
    }
}

impl ClubSource for Fixtures {
    fn name(&self) -> &'static str {
        "fixtures"
    }

    fn club<'a>(
        &'a self,
        request: &'a CircleRequest,
        _: Priority,
    ) -> SourceFuture<'a, CircleResponse> {
        let found = match request.lookup {
            CircleLookup::CircleId(id) => self.clubs.iter().find(|c| c.circle.circle_id == id),
            CircleLookup::ViewerId(id) => self.by_member(id),
        };
        let result = found.cloned().ok_or(SourceError::NotFound);
        Box::pin(async move { result })
    }

    fn list_clubs<'a>(
        &'a self,
        request: &'a ListCirclesRequest,
    ) -> SourceFuture<'a, CircleListResponse> {
        let first = request.page.unwrap_or(0) == 0;
        let response = CircleListResponse {
            circles: if first {
                self.clubs.iter().map(|c| c.circle.clone()).collect()
            } else {
                Vec::new()
            },
            total: CLUBS as i64,
            page: i64::from(request.page.unwrap_or(0)),
            limit: CLUBS as i64,
            total_pages: 1,
        };
        Box::pin(async move { Ok(response) })
    }

    fn rankings_monthly<'a>(
        &'a self,
        request: &'a RankingsMonthlyRequest,
    ) -> SourceFuture<'a, MonthlyRankingsResponse> {
        let rankings = request
            .query
            .as_deref()
            .and_then(|query| query.parse().ok())
            .and_then(|id| self.by_member(id).map(|club| (id, club)))
            .map(|(viewer_id, club)| MonthlyRanking {
                viewer_id,
                circle_id: Some(club.circle.circle_id),
                ..Default::default()
            })
            .into_iter()
            .collect();
        Box::pin(async move {
            Ok(MonthlyRankingsResponse {
                rankings,
                total_pages: 1,
                ..Default::default()
            })
        })
    }

    fn rankings_gains<'a>(
        &'a self,
        _: &'a RankingsGainsRequest,
    ) -> SourceFuture<'a, GainsRankingsResponse> {
        Box::pin(async { Err(SourceError::NotFound) })
    }

    /// Every fixture trainer has a profile, with the comment set through the control route.
    fn profile(&self, viewer_id: u64) -> SourceFuture<'_, ProfileResponse> {
        let result = self
            .by_member(viewer_id)
            .and_then(|club| {
                let member = club.members.iter().find(|m| m.viewer_id == viewer_id)?;
                let comment = self.comments.lock().ok()?.get(&viewer_id).cloned();
                let total = *member.daily_fans.last()?;
                Some(ProfileResponse {
                    trainer: ProfileTrainer {
                        account_id: viewer_id.to_string(),
                        name: member.trainer_name.clone().unwrap_or_default(),
                        comment,
                        leader_chara_dress_id: None,
                    },
                    circle: Some(ProfileCircle {
                        circle_id: club.circle.circle_id,
                        name: club.circle.name.clone(),
                    }),
                    fan_history: Some(ProfileFanHistory {
                        alltime: Some(ProfileAlltime {
                            total_fans: total,
                            active_days: DAYS,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                })
            })
            .ok_or(SourceError::NotFound);
        Box::pin(async move { result })
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    init_logging(LogFormat::from_env().unwrap_or_default(), None);
    let mut config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            tracing::error!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let comments = Comments::default();
    if let Some(settings) = config.discord.as_mut() {
        let port = match std::env::var("FIXTURE_DISCORD_PORT") {
            Ok(port) => match port.parse() {
                Ok(port) => port,
                Err(_) => {
                    tracing::error!("FIXTURE_DISCORD_PORT must be a port number, got {port:?}");
                    return ExitCode::FAILURE;
                }
            },
            Err(_) => DEFAULT_DISCORD_PORT,
        };
        match discord::start(port, settings, comments.clone()).await {
            Ok(()) => tracing::info!(port, "serving a mock Discord"),
            Err(error) => {
                tracing::error!("could not start the mock Discord: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    tracing::info!(clubs = CLUBS, "serving fixture clubs");
    match run(&config, Source::new(Fixtures::new(comments))).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("{error}");
            ExitCode::FAILURE
        }
    }
}
