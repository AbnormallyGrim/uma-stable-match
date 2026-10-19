# uma-stable-match-server

The API server for Uma Stable Match. For now it loads its settings, answers a health check and shuts down cleanly. It also includes the club scanner, which isn't scheduled yet. Later work adds the cache, the scan schedule and the recommendations API.

## Run

```bash
cp .env.example .env    # then add your uma.moe API key after UMAMOE_API_KEY=
cargo run -p uma-stable-match-server
```

Run these from the repository root. The server reads the first `.env` it finds in the directory it's started in or a parent directory. Real environment variables take precedence over `.env`.

## Configuration

| Variable | Default | Meaning |
|---|---|---|
| `UMAMOE_API_KEY` | none (required) | uma.moe API key. Create one in your uma.moe settings |
| `UMAMOE_BASE_URL` | `https://uma.moe` | API base URL override, for testing against a mock |
| `BIND_ADDR` | `127.0.0.1:3000` | Address and port to listen on. Use `0.0.0.0:3000` in a container |
| `RUST_LOG` | `info` | Log filter, such as `debug` or `uma_stable_match_server=debug` |
| `SCAN_RANK_FROM` | `80` | Best monthly rank to scan |
| `SCAN_RANK_TO` | `900` | Worst monthly rank to scan |
| `SCAN_MAX_DETAILS` | `80` | Most clubs to fetch member data for in each scan |

Blank values count as unset. A missing key, an invalid address, a scan setting that isn't a positive whole number, or `SCAN_RANK_FROM` above `SCAN_RANK_TO` stops the server at startup, with an error message and exit code 1. The key is never logged, never included in errors and is redacted in `Debug` output. `.env` is git-ignored; never commit it.

## Scanner

`scan` finds clubs worth recommending and fetches their member data, following the same rules as the original prototype:

1. List clubs by monthly rank, best first, 100 per page and up to `SCAN_RANK_TO`. Stop after a page whose last club is ranked `SCAN_RANK_TO` or worse, after a short or empty page, or after 100 pages.
2. Keep clubs ranked from `SCAN_RANK_FROM` to `SCAN_RANK_TO` that have open spots. A club with an unknown or zero member count is treated as full.
3. Sort climbers first, by last month's rank minus this month's. A club with no rank last month counts as a big climber. Clubs that climbed equally keep their list order.
4. Fetch member data for the first `SCAN_MAX_DETAILS` clubs and convert each one to a `Club`. If a club's details come back without club information, the listed details are used.

A club whose details fail to load is logged and skipped. A failed list request fails the whole scan. `Scan` returns the loaded clubs, the number of candidates and the number that failed.

With the client's default 500 ms rate limit, a default scan takes about 45 seconds: around 9 list pages plus 80 club lookups.

## Endpoints

| Method | Path | Response |
|---|---|---|
| `GET` | `/health` | `200 {"status":"ok"}` |

## Shutdown

Ctrl+C or SIGTERM (as sent by `docker stop`) stops accepting new connections, lets in-flight requests finish, and then exits.
