use super::*;
use uma_stable_match_umamoe_client::{Circle, CircleLookup};

struct Fixed;

impl ClubSource for Fixed {
    fn name(&self) -> &'static str {
        "made-up"
    }

    fn club<'a>(
        &'a self,
        request: &'a CircleRequest,
        _: Priority,
    ) -> SourceFuture<'a, CircleResponse> {
        Box::pin(async move {
            match request.lookup {
                CircleLookup::CircleId(42) => Ok(CircleResponse {
                    circle: Circle {
                        circle_id: 42,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                _ => Err(SourceError::NotFound),
            }
        })
    }

    fn list_clubs<'a>(&'a self, _: &'a ListCirclesRequest) -> SourceFuture<'a, CircleListResponse> {
        Box::pin(async { Ok(CircleListResponse::default()) })
    }

    fn rankings_monthly<'a>(
        &'a self,
        _: &'a RankingsMonthlyRequest,
    ) -> SourceFuture<'a, MonthlyRankingsResponse> {
        Box::pin(async {
            Err(SourceError::Unavailable {
                origin: "made-up",
                message: "made-up source is down".into(),
            })
        })
    }

    fn rankings_gains<'a>(
        &'a self,
        _: &'a RankingsGainsRequest,
    ) -> SourceFuture<'a, GainsRankingsResponse> {
        Box::pin(async { Ok(GainsRankingsResponse::default()) })
    }
}

#[tokio::test]
async fn other_sources_plug_in() {
    let source = Source::new(Fixed);
    let found = source
        .club(&CircleRequest::by_circle_id(42), Priority::Interactive)
        .await
        .unwrap();
    assert_eq!(found.circle.circle_id, 42);

    let missing = source
        .club(&CircleRequest::by_circle_id(7), Priority::Background)
        .await
        .unwrap_err();
    assert!(missing.is_not_found());

    let down = source
        .rankings_monthly(&RankingsMonthlyRequest::for_viewer(1))
        .await
        .unwrap_err();
    assert_eq!(down.origin(), "made-up");
    assert_eq!(down.status(), None);
    assert_eq!(down.summary(), "made-up source is down");
}

#[test]
fn not_found_counts_as_404() {
    let error = SourceError::NotFound;
    assert_eq!(error.status(), Some(StatusCode::NOT_FOUND));
    assert_eq!(error.origin(), "source");
    assert!(error.is_not_found());
}

#[test]
fn uma_moe_errors_keep_their_origin_and_status() {
    let error = SourceError::from(Error::Status {
        status: StatusCode::NOT_FOUND,
        body: None,
    });
    assert_eq!(error.origin(), "umamoe");
    assert!(error.is_not_found());
    assert_eq!(error.to_string(), "uma.moe returned 404 Not Found");
}

#[test]
fn uma_moe_summaries_hide_details() {
    let error = SourceError::from(Error::RateLimited {
        retry_in: std::time::Duration::from_secs(60),
    });
    assert_eq!(error.summary(), "uma.moe asked to slow down");
    let error = SourceError::from(Error::InvalidRequest("made-up detail".into()));
    assert_eq!(error.summary(), "internal error");
}
