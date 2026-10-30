# uma-stable-match-scoring

Ranks Umamusume: Pretty Derby clubs by how well they suit a trainer. Club and member data goes in, scores come out. The crate has no network or storage code.

## Usage

```rust
use uma_stable_match_scoring::{daily_pace, score_clubs, Club, ScoringOptions};

// Your cumulative fan count for each day of the month.
let my_fans = [150_000_000, 157_500_000, 165_000_000];
let pace = daily_pace(&my_fans).expect("at least three days of data");

// Load clubs and their member snapshots from your data source.
let clubs: Vec<Club> = Vec::new();

let options = ScoringOptions::default().with_balance_weight(0.5);
for scored in score_clubs(pace, &clubs, &options).expect("valid pace and options") {
    println!("{}: place {} of {}, score {:.3}",
        scored.club.name, scored.your_place, scored.club_size, scored.score);
}
```

Enable the `serde` feature to deserialise `Club` and `MemberSnapshot` directly from JSON:

```toml
uma-stable-match-scoring = { version = "0.1", features = ["serde"] }
```

## Minimum supported Rust version

Rust 1.71, with or without the `serde` feature.

## Versioning

The crate follows [Semantic Versioning](https://semver.org/). While it's below 1.0, the minor version marks breaking changes: 0.1 → 0.2 is breaking, 0.1.0 → 0.1.1 isn't.

| Change | Release |
|---|---|
| Removing or changing a public item | Breaking |
| Changing how scores are calculated, or which clubs are left out | Breaking |
| Raising the minimum Rust version | Breaking |
| Adding a field to a `#[non_exhaustive]` type, or a variant to a `#[non_exhaustive]` enum | Non-breaking |
| Adding a public item | Non-breaking |
| Fixing a bug that doesn't change scores | Non-breaking |

Every release is listed in the [changelog](https://github.com/AbnormallyGrim/uma-stable-match/blob/main/backend/scoring/CHANGELOG.md).

## Input data

A `MemberSnapshot` holds one member's **cumulative** fan count for each day of the month, starting at day 1:

| Value | Meaning |
|---|---|
| Positive | Fan count while in this club |
| `0` | No data for that day (before joining, or after leaving) |
| Negative | Fan count recorded while in a previous club |

A `Club` holds the club's details and its member snapshots for the month.

`Club`, `MemberSnapshot` and `ScoringOptions` are `#[non_exhaustive]`, so new fields can be added without a breaking release. Build them with a constructor and set the other fields directly:

```rust
use uma_stable_match_scoring::{Club, JoinStyle, MemberSnapshot, ScoringOptions};

let mut member = MemberSnapshot::new(1, vec![100_000_000, 105_000_000, 110_000_000]);
member.year = Some(2026);
member.month = Some(3);

let mut club = Club::new(1, "Example Club");
club.member_count = Some(25);
club.join_style = Some(JoinStyle::Open);
club.members.push(member);

let options = ScoringOptions::default()
    .with_balance_weight(0.5)
    .with_include_closed(true);
```

`JoinStyle`, `ScoringError`, `Balance` and `ScoredClub` are also `#[non_exhaustive]`. A `match` on `JoinStyle` or `ScoringError` needs a `_` arm, and `Balance` and `ScoredClub` can only be read, not built.

## How scoring works

### Pace

`daily_pace` skips days without data and counts fans from a previous club:

```text
pace = (latest value − first value) ÷ (days with data − 1)
```

At least 3 days of data are needed.

### Blended pace

Early in a month there are too few days for a reliable pace. `blended_pace` combines this month's pace with a prior, such as the member's pace last month, and trusts this month more with each day of data:

```text
d     = days with data − 1
Z     = d ÷ (d + k)
pace  = Z × this month's pace + (1 − Z) × prior
```

With no gains yet (`d` = 0) it's the prior. Without a prior it's `daily_pace`. A larger `k` keeps more weight on the prior for longer, and `k` = 0 ignores it once there's a gain. This is credibility weighting, the pattern baseball projection systems such as Marcel use. A backtest on real clubs found `k` = 1 suits club medians and `k` = 4 a single trainer's pace.

`blended_median` is the club median over current members' blended paces, with a prior looked up for each member by trainer ID. Unlike the fit's median, it includes members with fewer than 3 days of data when they have a prior. Negative paces count as 0, and it needs at least 8 paces.

```rust
use uma_stable_match_scoring::blended_pace;

// 10 fans a day over 3 gains, against a prior of 40 a day: Z = 3 ÷ 4.
assert_eq!(blended_pace(&[100, 110, 120, 130], Some(40.0), 1.0), Some(17.5));
```

### Current members

`current_members` decides who is still in a club:

1. Only snapshots from the latest month present are used.
2. The club's latest data day is the last day on which any member has data.
3. A member is current if their value on that day is positive.

### Place and fit

```text
club size  = min(official member count, 29) + 1        (you included, never above 30)
your place = 1 + current members with a higher pace
fit        = |your place ÷ club size − 0.4| + 0.3 × |club median ÷ your pace − 0.85|
```

Lower fit is better. It's lowest when you'd sit about 40% of the way down the club and the club median is a little below your pace.

Members with fewer than 3 days of data are counted in the club size but can't be ranked, so they're placed below you. A club needs at least 8 rankable members to be scored.

### Balance

`club_balance` measures how evenly current members contribute. Members with fewer than 3 days of data are left out.

| Measure | Definition |
|---|---|
| Passenger | Gaining under 1M fans/day, or under 25% of the club median, whichever is higher |
| Gini | 0 = everyone contributes equally, 1 = one member does everything |
| Bottom quarter | Median daily gain of the lowest-contributing quarter |
| Top-3 share | Share of the club's daily gain from its three biggest contributors |

```text
balance = 100 × (0.5 × (1 − gini) + 0.5 × (1 − passenger share))     rounded, halves to even
```

### Final score

With `ScoringOptions::balance_weight` set to `w`:

```text
score = fit + w × (1 − balance ÷ 100)
```

Without it, the score is the fit. `score_clubs` returns clubs sorted by score, lowest (best) first.

### Clubs that are left out

- Fewer than 8 rankable current members
- Closed clubs, unless `ScoringOptions::include_closed` is set
- Clubs without a balance score, when balance scoring is on

## Errors

`score_clubs` and `score_club` return a `ScoringError` when:

- the pace isn't a finite number above zero
- the balance weight is outside 0 to 1

## Tiers

`tier_label` converts a club's tier code to its label:

```rust
use uma_stable_match_scoring::tier_label;

assert_eq!(tier_label(1), Some("D"));
assert_eq!(tier_label(8), Some("A+"));
assert_eq!(tier_label(11), Some("SS"));
```
