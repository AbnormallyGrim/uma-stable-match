# Uma Stable Match

**Find your stable.** Uma Stable Match recommends Umamusume: Pretty Derby clubs that suit you: clubs where your fan pace fits, the club is climbing, and members pull their weight.

![Stable Match results page: a summary of your pace and the clubs analysed, followed by ranked club recommendations showing each club's tier, rank trend, open spots, balance score and your expected place](docs/images/demo.png)

<sub>Example results page, shown with sample data.</sub>

> [!IMPORTANT]
> Unofficial, non-commercial fan tool. Not affiliated with uma.moe or Cygames, Inc. See [Disclaimer](#disclaimer).

> [!NOTE]
> Uma Stable Match is in early development. Sections marked *(coming soon)* describe features that aren't available yet.

## Contents

- [Overview](#overview)
- [Features](#features)
- [Requirements](#requirements)
- [Installation](#installation)
- [Usage](#usage)
- [Understanding your results](#understanding-your-results)
- [Configuration](#configuration)
- [API](#api)
- [Tips for switching clubs](#tips-for-switching-clubs)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)
- [Disclaimer](#disclaimer)

## Overview

Club leaderboards show how big a club is, not whether it's right for you. Uma Stable Match compares your daily fan gain with the members of every club that has an open spot, and ranks the clubs where you'd be a solid member: secure enough not to be cut, without carrying everyone else.

Enter your trainer ID and you'll get a ranked list of clubs, each with its tier, rank trend, open spots, join style, how you'd compare with its members, and a link to the club's page on [uma.moe](https://uma.moe).

## Features

- **Personal recommendations** based on your fan pace this month
- **Open clubs first:** clubs that aren't recruiting are left out unless you ask for them
- **Current members only:** members who have left a club don't skew its numbers
- **Balance scoring:** optionally ranks clubs lower when many members barely contribute
- **Club details at a glance:** tier, rank and trend, open spots, join style, playstyle and description
- **Search and sort** all results
- **Light and dark themes**
- **Self-hostable** with Docker

## Requirements

- A free [uma.moe](https://uma.moe) account and API key ([how to get one](#1-get-a-umamoe-api-key))
- Your 12-digit trainer ID (in-game: **Menu → Profile**)
- To run it yourself: [Docker Desktop](https://www.docker.com/products/docker-desktop/)

## Installation

*(coming soon)*

### 1. Get a uma.moe API key

1. Sign in at [uma.moe/login](https://uma.moe/login) with Google or Discord.
2. Go to [uma.moe/settings](https://uma.moe/settings) → **API Keys** and create a key.
3. Copy the key straight away. It's only shown once.

> [!CAUTION]
> Treat your API key like a password. Don't commit it, share it, or post it in issues. If it leaks, revoke it in uma.moe's settings and create a new one.

### 2. Download and configure

```bash
git clone https://github.com/AbnormallyGrim/uma-stable-match.git
cd uma-stable-match
cp .env.example .env
```

Open `.env` in a text editor and paste your key:

```env
UMAMOE_API_KEY=your-key-here
```

### 3. Start it

```bash
docker compose up -d
```

Open **<http://localhost:8080>** in your browser.

To stop it:

```bash
docker compose down
```

## Usage

*(coming soon)*

1. Open Uma Stable Match in your browser.
2. Enter your **trainer ID** and select **Find clubs**.
3. Optionally turn on **Balance scoring** and adjust its strength (0–1). Higher values push clubs with many low contributors further down.
4. Browse **Top picks**, or use **All results** to search and sort every club.
5. Select a club's name to open its uma.moe page in a new tab, then contact the leader.

Club data refreshes automatically every 3 hours by default. The page shows when it was last updated.

## Understanding your results

![All results table listing every analysed club with its tier, rank, last month's rank, open spots, join style, median daily gain, your place, balance score, passengers, top-3 share and fit score, with search, a hide-closed-clubs filter and sortable columns](docs/images/demo-results-table.png)

<sub>The All results table, shown with sample data. Search by name, hide closed clubs, or select a column to sort.</sub>

| Field | Meaning |
|---|---|
| **Fit** | How well the club suits you. **Lower is better.** |
| **Your place** | Where you'd rank among the club's current members, for example *8 of 30* |
| **Median / day** | The typical member's daily fan gain this month |
| **Rank** | The club's monthly rank, with its change since last month |
| **Tier** | The club's rank tier, from D to SS |
| **Join style** | *Open* (join instantly), *Approval* (leader accepts you) or *Closed* (not recruiting) |
| **Balance** | 0–100, how evenly members contribute. **Higher is better.** |
| **Passengers** | Members gaining under 1M fans/day, or under a quarter of the club median |
| **Top 3 share** | Percentage of the club's fans coming from its three biggest contributors |

### How clubs are ranked

**Your pace** is your average daily fan gain this month.

**Fit** is best when you'd sit roughly 25–60% of the way down the club's members and the club's median is a little below your pace:

```
fit = |your place ÷ club size − 0.4| + 0.3 × |club median ÷ your pace − 0.85|
```

**Balance** combines how evenly fans are spread across members (the Gini coefficient) with the share of passengers:

```
balance = 100 × (0.5 × (1 − gini) + 0.5 × (1 − passenger share))
```

With balance scoring on, the final score is:

```
score = fit + weight × (1 − balance ÷ 100)
```

Only **current members** count. A member counts as current if they have data on the club's most recent day, the same rule uma.moe uses. Members with fewer than 3 days of data are too new to rank, so they're placed below you and left out of the balance score.

## Configuration

*(coming soon)* Set these in `.env`.

| Variable | Default | Description |
|---|---|---|
| `UMAMOE_API_KEY` | *(required)* | Your uma.moe API key |
| `HOST` | `127.0.0.1` | IP address the web page and API listen on. The Docker setup uses `0.0.0.0` |
| `PORT` | `8080` | Port the web page and API run on |
| `SCAN_INTERVAL_MINUTES` | `180` | How often club data refreshes, in minutes |
| `SCAN_RANK_FROM` | `80` | Best monthly rank to include |
| `SCAN_RANK_TO` | `900` | Worst monthly rank to include |
| `SCAN_MAX_DETAILS` | `80` | Most clubs to analyse in each refresh |
| `TRAINER_CACHE_MINUTES` | `10` | How long a trainer's pace is reused before refreshing |
| `RATE_LIMIT_PER_MINUTE` | `30` | Requests allowed per visitor per minute |
| `TRAINER_LOOKUPS_PER_MINUTE` | `10` | Trainer ID lookups allowed per visitor per minute, not counting cached ones |
| `TRUST_FORWARDED_FOR` | `false` | Set to `true` behind a reverse proxy, so visitors are told apart by their own address |
| `REDIS_URL` | *(none)* | Optional. Share one cache between several servers |

## API

*(coming soon)* The web page is built on a JSON API you can also call directly.

### Get recommendations

```
GET /api/v1/recommendations
```

| Parameter | Required | Default | Description |
|---|---|---|---|
| `trainer_id` | yes, or `pace` | | Your 12-digit trainer ID |
| `pace` | yes, or `trainer_id` | | Your daily fan gain, if you'd rather not look up a trainer |
| `balance` | no | `false` | Include balance scoring |
| `balance_weight` | no | `0.5` | Balance strength, 0–1. Setting it turns balance on. |
| `include_closed` | no | `false` | Include closed clubs |
| `limit` | no | `50` | Maximum number of clubs returned |

Example:

```bash
curl "http://localhost:8080/api/v1/recommendations?trainer_id=123456789012&balance_weight=0.6&limit=1"
```

```json
{
  "trainer": { "trainer_id": 123456789012, "pace_per_day": 7620000 },
  "options": { "balance_weight": 0.6, "include_closed": false, "limit": 1 },
  "updated_at": "2026-10-19T08:00:00Z",
  "total": 42,
  "clubs": [
    {
      "circle_id": 100000001,
      "name": "Example Club",
      "url": "https://uma.moe/circles/100000001",
      "tier": "A+",
      "join_style": "approval",
      "policy": 2,
      "member_count": 29,
      "open_spots": 1,
      "monthly_rank": 111,
      "last_month_rank": 127,
      "comment": "Example description",
      "your_place": 12,
      "club_size": 30,
      "ranked_members": 28,
      "median_per_day": 5650000,
      "fit": 0.104,
      "score": 0.248,
      "balance": {
        "score": 76,
        "gini": 0.18,
        "passengers": 0,
        "counted": 28,
        "bottom_quarter_per_day": 3900000,
        "top3_share": 0.34
      }
    }
  ]
}
```

`total` is how many clubs were scored before `limit` was applied. The [server README](backend/server/README.md#recommendations) describes every field and error.

### Other endpoints

| Endpoint | Returns |
|---|---|
| `GET /api/v1/status` | When club data was last refreshed, how many clubs are cached, and the last scan error |
| `GET /api/v1/health` | `200 OK` while the server is running |

## Tips for switching clubs

- **14-day rule:** you need at least 14 days in a club during a month to receive its monthly reward, so switch early in the month.
- **24-hour cooldown:** after leaving a club you can't join another for 24 hours. Players report that being removed by the leader avoids this.
- **Month-start lock:** from 10:00 to 14:59 UTC on the 1st, you can't leave, apply to or invite to clubs.
- **Check activity first:** review members' activity scores on uma.moe before applying.

These rules can change, so check in-game if something doesn't match.

## Development

*(coming soon)*

### Prerequisites

- [Rust](https://rustup.rs) (latest stable)
- [Node.js](https://nodejs.org) 22.22.2 or newer (or 24.15 or newer)
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) (optional, for Redis and full-stack runs)

### Project layout

```
backend/
  scoring/         Club scoring (no network or storage dependencies)
  umamoe-client/   uma.moe API client
  server/          API server, scheduled club scanner and cache
frontend/          Web interface (Svelte + TypeScript)
```

### Run locally

```bash
# Backend
cargo run -p uma-stable-match-server

# Frontend (separate terminal)
cd frontend
npm install
npm run dev
```

Open <http://localhost:5173>. The frontend dev server forwards `/api` requests to the backend. See the [frontend README](frontend/README.md).

### Test

```bash
cargo test
cd frontend && npm test
```

### Using the scoring crate on its own

The `scoring` crate has no network or storage dependencies. If you already have club and member data, you can score it directly. See the [scoring crate documentation](backend/scoring/README.md) for usage, input data and the scoring rules.

To browse the API documentation locally:

```bash
cargo doc -p uma-stable-match-scoring --all-features --open
```

## Contributing

Bug reports, ideas and pull requests are welcome. See the [Contributing guide](CONTRIBUTING.md) for the workflow, branch naming and checks, and ask questions in [Discussions](https://github.com/AbnormallyGrim/uma-stable-match/discussions).

To report a security issue, follow the [Security Policy](SECURITY.md) rather than opening a public issue.

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

## License

[MIT](LICENSE) © 2026 AbnormallyGrim

The license covers this project's source code only, not Cygames' artwork or trademarks.

## Disclaimer

Uma Stable Match is an **unofficial, non-commercial fan project** and is **not affiliated with, endorsed by, or sponsored by uma.moe or Cygames, Inc.**

- **Umamusume: Pretty Derby** and all related names, characters, artwork and trademarks belong to **Cygames, Inc.** Game artwork shown by this tool, including club tier icons, is © Cygames, Inc. and is used non-commercially in line with Cygames' [Fan-Created Content Guidelines](https://umamusume.com/fan-createdguide).
- Club and trainer data comes from the [uma.moe](https://uma.moe) API and may be delayed or incomplete.
- Recommendations are estimates to help you decide, not judgements of any player or club.
- This software is provided "as is", without warranty of any kind.
- Uma Stable Match is free, with no ads or paid features.

Rights holders with concerns can [open an issue](https://github.com/AbnormallyGrim/uma-stable-match/issues), and they'll be addressed promptly.
