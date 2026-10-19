# uma-stable-match-server

The API server for Uma Stable Match. Later work adds the club scanner, the cache and the recommendations API. For now the server loads its settings, answers a health check and shuts down cleanly.

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

Blank values count as unset. A missing key or an invalid address stops the server at startup, with an error message and exit code 1. The key is never logged, never included in errors and is redacted in `Debug` output. `.env` is git-ignored; never commit it.

## Endpoints

| Method | Path | Response |
|---|---|---|
| `GET` | `/health` | `200 {"status":"ok"}` |

## Shutdown

Ctrl+C or SIGTERM (as sent by `docker stop`) stops accepting new connections, lets in-flight requests finish, and then exits.
