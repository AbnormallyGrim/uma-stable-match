# uma-stable-match-umamoe-client

Types for the [uma.moe](https://uma.moe) API v4. They hold Umamusume: Pretty Derby club and member fan data and convert it into the types used by [`uma-stable-match-scoring`](../scoring/README.md).

This is an unofficial client. It isn't affiliated with uma.moe or Cygames. The HTTP client itself isn't implemented yet.

## Response types

| Type | Endpoint |
|---|---|
| `CircleResponse` | `GET /api/v4/circles`: a club, its members' daily fans and its tier |
| `CircleListResponse` | `GET /api/v4/circles/list`: one page of clubs, without members or tier |

The types follow the uma.moe OpenAPI schema but are lenient. Missing fields take their default, `null` becomes `None`, and unknown fields are ignored. Timestamps are kept as ISO 8601 strings.

`CircleListResponse` pages from 0. `has_next_page` reports whether a later page exists.

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
