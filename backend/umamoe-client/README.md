# uma-stable-match-umamoe-client

An async client for the [uma.moe](https://uma.moe) API v4. It fetches Umamusume: Pretty Derby club and member fan data and converts it into the types used by [`uma-stable-match-scoring`](../scoring/README.md).

This is an unofficial client. It isn't affiliated with uma.moe or Cygames.

## Usage

```rust,no_run
use uma_stable_match_scoring::Club;
use uma_stable_match_umamoe_client::{
    ApiKey, CircleRequest, CircleSort, Client, ClientConfig, ListCirclesRequest, SortDir,
};

# async fn run() -> Result<(), uma_stable_match_umamoe_client::Error> {
// Read the key from your environment or config; never hard-code it.
let key = std::env::var("UMAMOE_API_KEY").expect("UMAMOE_API_KEY is set");
let client = Client::new(ClientConfig::new(ApiKey::new(key)))?;

// One page of clubs, best monthly rank first.
let request = ListCirclesRequest {
    sort_by: Some(CircleSort::MonthlyRank),
    sort_dir: Some(SortDir::Asc),
    ..Default::default()
};
let page = client.list_circles(&request).await?;

// Full details and member fan data for one club, ready for scoring.
if let Some(circle) = page.circles.first() {
    let response = client.circle(&CircleRequest::by_circle_id(circle.circle_id)).await?;
    let club = Club::from(response);
    println!("{} has {} member snapshots", club.name, club.members.len());
}
# Ok(())
# }
```

## Endpoints

| Method | Endpoint | Returns |
|---|---|---|
| `Client::circle` | `GET /api/v4/circles` | `CircleResponse`: a club, its members' daily fans and its tier |
| `Client::list_circles` | `GET /api/v4/circles/list` | `CircleListResponse`: one page of clubs, without members or tier |

`CircleRequest` looks a club up by `circle_id` or by a member's `viewer_id`. Without `for_month`, the API returns the current game month, which starts on the 2nd (JST). A month outside 1 to 12 is rejected before any request is sent.

`ListCirclesRequest` filters by name, minimum members and worst monthly rank, and sorts by name, member count, monthly rank or monthly points. Fields left as `None` aren't sent, so the API's defaults apply. Pages start at 0: use `has_next_page` on the response and `ListCirclesRequest::page` to walk them.

## Response types

| Type | Endpoint |
|---|---|
| `CircleResponse` | `GET /api/v4/circles`: a club, its members' daily fans and its tier |
| `CircleListResponse` | `GET /api/v4/circles/list`: one page of clubs, without members or tier |

The types follow the uma.moe OpenAPI schema but are lenient. Missing fields take their default, `null` becomes `None`, and unknown fields are ignored. Timestamps are kept as ISO 8601 strings.

## Converting to scoring types

| From | To | Notes |
|---|---|---|
| `CircleResponse` | `Club` | Includes members and tier (`club_rank`) |
| `Circle` | `Club` | No members or tier, because the list endpoint doesn't return them |
| `CircleMember` | `MemberSnapshot` | `daily_fans` is passed through unchanged |

A code that is out of range or unknown becomes `None`. This covers negative counts, join styles other than 1 to 3, and tiers above 255.

```rust
use uma_stable_match_scoring::Club;
use uma_stable_match_umamoe_client::CircleResponse;

let json = r#"{"circle": {"circle_id": 1, "name": "Example Club", "join_style": 1}, "club_rank": 6}"#;
let response: CircleResponse = serde_json::from_str(json).unwrap();
let club = Club::from(response);
assert_eq!(club.tier(), Some("B+"));
```

## Authentication

The key is sent as the `X-API-Key` header and marked sensitive. `ApiKey`, `ClientConfig` and `Client` all redact it in `Debug` output, and it never appears in error messages. Keep it in `.env` or your deployment's secret store.

## Rate limiting

Requests are spaced at least `min_interval` apart (500 ms by default) and go out in the order they were made. All clones of a `Client` share one limiter, so a scanner running many tasks still sends one request per slot. Idle time isn't saved up for bursts. Set `min_interval` to zero to turn the limit off.

## Errors

Errors are returned straight away; requests aren't retried.

| Variant | When |
|---|---|
| `Config` | Invalid base URL or API key |
| `InvalidRequest` | Invalid parameters, such as a month outside 1 to 12 |
| `Status` | Non-success status, with the parsed `{error, status, details}` body when the API sent one |
| `Transport` | The request couldn't be sent or timed out, or the response couldn't be read |
| `Decode` | The response body didn't match the expected shape |
