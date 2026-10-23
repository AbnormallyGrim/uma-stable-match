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
| `Client::rankings_gains` | `GET /api/v4/rankings/gains` | `GainsRankingsResponse`: one page of trainers' fan gains over the last 3, 7 and 30 days |

`CircleRequest` looks a club up by `circle_id` or by a member's `viewer_id`. Without `for_month`, the API returns the current game month, which starts on the 2nd (JST). A month outside 1 to 12 is rejected before any request is sent.

`ListCirclesRequest` filters by name, minimum members and worst monthly rank, and sorts by name, member count, monthly rank or monthly points. Fields left as `None` aren't sent, so the API's defaults apply. Pages start at 0: use `has_next_page` on the response and `ListCirclesRequest::page` to walk them.

`RankingsGainsRequest` sorts by 3-, 7- or 30-day gain and pages the same way, up to 100 trainers per page. The gains are rolling windows that cross month boundaries, so they're still available in the first days of a game month. `query` matches a trainer ID exactly but also matches parts of trainer and club names, so a search for an ID can return other trainers too. Use `GainsRankingsResponse::viewer` to pick the exact one:

```rust
use uma_stable_match_umamoe_client::GainsRankingsResponse;

let json = r#"{"rankings": [
    {"viewer_id": 100000012, "gain_7d": 21000000},
    {"viewer_id": 100000001, "gain_7d": 14000000}
]}"#;
let response: GainsRankingsResponse = serde_json::from_str(json).unwrap();
let me = response.viewer(100000001).unwrap();
assert_eq!(me.gain_7d, Some(14_000_000));
```

## Response types

| Type | Endpoint |
|---|---|
| `CircleResponse` | `GET /api/v4/circles`: a club, its members' daily fans and its tier |
| `CircleListResponse` | `GET /api/v4/circles/list`: one page of clubs, without members or tier |
| `GainsRankingsResponse` | `GET /api/v4/rankings/gains`: one page of `GainsRanking` rows |

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

Requests are spaced at least `min_interval` apart (500 ms by default). Retries count too. All clones of a `Client` share one limiter, so a scanner running many tasks still sends one request per slot. Idle time isn't saved up for bursts. Set `min_interval` to zero to turn the limit off.

Each client sends its requests in one of two lanes:

| `Priority` | Use for |
|---|---|
| `Interactive` (default) | Requests someone is waiting for |
| `Background` | Prefetching and other work that can wait |

When a slot opens, the oldest waiting interactive request takes it. Background requests only get a slot when no interactive request is waiting. Within a lane, requests go out in the order they were made. A request that's cancelled while waiting gives up its place.

`Client::with_priority` returns a clone that uses another lane but shares the same limiter, so both lanes together stay within `min_interval`:

```rust,no_run
use uma_stable_match_umamoe_client::{ApiKey, Client, ClientConfig, Priority};

# fn run() -> Result<(), uma_stable_match_umamoe_client::Error> {
let key = std::env::var("UMAMOE_API_KEY").expect("UMAMOE_API_KEY is set");
let client = Client::new(ClientConfig::new(ApiKey::new(key)))?;
let prefetch = client.with_priority(Priority::Background);
# Ok(())
# }
```

A steady stream of interactive requests can hold background ones back indefinitely. That's intended: prefetching should never slow down a visitor.

### Backing off

When uma.moe answers 429, the whole client slows down, not just the refused request:

- **`Retry-After` pauses every request.** No request in either lane is sent until it has passed, however long uma.moe asks for.
- **The rate halves on each 429**, down to one request per `max_interval`. Further 429s during the same pause don't halve it again, so one episode of throttling counts once.
- **It recovers additively.** After each `recovery_every` without a 429, counted from the end of any pause, the rate goes up by `recovery_step` requests per second, back up to one per `min_interval`.

This is additive increase, multiplicative decrease (AIMD), the same scheme TCP uses for congestion: a one-off 429 dips the rate briefly, while repeated 429s settle it just below what the API accepts. The settings are in `ClientConfig::backoff`:

| `BackoffConfig` | Default | Meaning |
|---|---|---|
| `max_interval` | 30 s | Slowest pace: one request this often |
| `recovery_step` | 0.05 | Requests per second added after each quiet period |
| `recovery_every` | 10 s | Length of a quiet period |

With the defaults, recovering from a quarter of the normal rate takes about 5 minutes. With `min_interval` set to zero, only the `Retry-After` pause applies.

Each 429 is logged as a warning through [`tracing`](https://docs.rs/tracing), with `endpoint`, `lane` and `retry_after_secs` when uma.moe sent `Retry-After`.

### Not waiting too long

A long `Retry-After` pause holds every request, which suits background work but not someone waiting for a page. `Client::with_max_pause` returns a clone that gives up instead: if the pause runs longer than its limit, before sending or before retrying, the call returns `Error::RateLimited` with the time left. Clones without a limit, the default, wait out every pause.

## Retries

These are retried:

- Status 429, 500, 502, 503 and 504
- Timeouts and connection errors

Other errors are returned straight away. That includes other 4xx responses, invalid JSON and other transport errors. When retries run out, the last error is returned.

| Setting | Default | Meaning |
|---|---|---|
| `max_retries` | 3 | Retries after the first attempt |
| `base_delay` | 500 ms | First retry delay, doubled each time |
| `max_delay` | 30 s | Longest backoff between attempts |

The wait before retry *n* (from 0) is:

```text
exp  = min(base_delay × 2ⁿ, max_delay)
wait = exp ÷ 2 + random(0, exp ÷ 2)
```

When the response has a `Retry-After` header in seconds, that value is used instead, in full; a client with a [max pause](#not-waiting-too-long) gives up rather than wait longer than it. A 429 also slows the whole client down; see [Backing off](#backing-off). HTTP-date values are ignored. `timeout` applies to each attempt, not to the whole call.

## Errors

| Variant | When |
|---|---|
| `Config` | Invalid base URL or API key |
| `InvalidRequest` | Invalid parameters, such as a month outside 1 to 12 |
| `Status` | Non-success status, with the parsed `{error, status, details}` body when the API sent one |
| `RateLimited` | uma.moe asked for a longer `Retry-After` pause than `with_max_pause` allows. `status()` returns 429 |
| `Transport` | The request couldn't be sent or timed out, or the response couldn't be read |
| `Decode` | The response body didn't match the expected shape |

`Error::origin` says where an error came from: `"umamoe"` for `Status`, `RateLimited` and `Decode`, `"network"` for `Transport`, and `"server"` for `Config` and `InvalidRequest`. Each failed call, after any retries, also logs one `uma.moe request failed` warning through [`tracing`](https://docs.rs/tracing) with the endpoint, lane, origin, status, uma.moe's error message, attempts and a stack trace (captured when `RUST_LIB_BACKTRACE=1`). 404s aren't logged.

## Metrics

Each call records metrics through the [`metrics`](https://docs.rs/metrics) facade. They cost nothing unless the application installs a recorder, such as the server's Prometheus exporter.

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `umamoe_requests_total` | counter | `endpoint`, `lane`, `status` | Attempts, so each retry counts as another one. `status` is the HTTP status code, or `timeout`, `connect_error` or `transport_error` when no response came back |
| `umamoe_request_duration_seconds` | histogram | `endpoint`, `lane` | Time from sending an attempt to its response or error |
| `umamoe_queue_wait_seconds` | histogram | `lane` | Time an attempt waited for its slot under `min_interval` |
| `umamoe_allowed_rate` | gauge | | Requests per second the client currently allows, lowered by [backing off](#backing-off). Absent when `min_interval` is zero |

`endpoint` is the API path, such as `/api/v4/circles`, never the full URL, so trainer IDs and other query values don't appear in labels. `lane` is `interactive` or `background`.

## Parity tests

`tests/parity.rs` checks that the scoring crate gives the same results as the original prototype, the Python script this project grew from. It loads `tests/fixtures/parity/clubs.json` through `CircleResponse`, converts it to `Club` and compares the results with `expected.json`:

- Which clubs are scored and in what order
- Fit, score and median within 1e-9 (relative)
- Place, club size and ranked members exactly
- Balance score, Gini, passengers, bottom quarter and top-three share

It covers two paces, each without balance scoring and with weights 0.5 and 1. Closed clubs are included, because the prototype scores them.

The clubs, trainers and fan counts are made up. They cover departed members, fans from a previous club, new joiners, older months, too few members, a closed club, passengers, a full club, an unknown member count, a falling fan total and an even number of members.

`expected.json` was recorded by running the prototype's own scoring functions on these clubs. The prototype isn't in this repository, so the expected values can't be regenerated from here. If a scoring rule changes on purpose, update `expected.json` in the same change and say why.
