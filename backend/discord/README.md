# uma-stable-match-discord

The Discord side of club applications: a REST client that calls Discord as the bot, the endpoint Discord sends interactions to, such as button clicks, and Sign in with Discord. The server mounts it when Discord is configured; see [Discord](../server/README.md#discord-applications) in the server README.

There's no gateway connection and no always-on bot process. Everything the bot does is a REST call, and everything Discord tells it arrives as a signed HTTP request.

## Usage

```rust,no_run
use serde_json::Value;
use uma_stable_match_discord::{
    router, ApplicationId, BotToken, ClientSecret, Discord, DiscordConfig, PublicKey, PublicUrl,
    SigningKey,
};

# async fn run() -> Result<(), uma_stable_match_discord::Error> {
// Read these from your environment; never hard-code them.
let env = |name| std::env::var(name).unwrap_or_default();
let config = DiscordConfig::new(
    ApplicationId::parse(&env("DISCORD_APP_ID")).expect("a valid application ID"),
    PublicKey::parse(&env("DISCORD_PUBLIC_KEY")).expect("a valid public key"),
    BotToken::parse(&env("DISCORD_BOT_TOKEN")).expect("a bot token"),
    ClientSecret::parse(&env("DISCORD_CLIENT_SECRET")).expect("a client secret"),
    SigningKey::parse(&env("TOKEN_SIGNING_KEY")).expect("a signing key of 32+ characters"),
    PublicUrl::parse(&env("PUBLIC_URL")).expect("the site's address"),
);
let discord = Discord::new(config)?;

// REST calls as the bot.
let guild: Value = discord.rest().get("/guilds/123").await?;

// The interactions endpoint and sign-in routes, to merge into an axum app.
let app = router(discord);
# let _ = (guild, app);
# Ok(())
# }
```

`DiscordConfig::api_base` points the client somewhere else, such as a mock server in tests. The bot token and client secret are redacted in `Debug` output.

## REST client

`Rest` has `get`, `post`, `put`, `patch` and `delete`. Each takes a path under the API base, such as `/channels/123/messages`, and deserializes the JSON response. An empty response, such as a 204, reads as `null`, so `()` works for those. Requests send `Authorization: Bot <token>` and the `DiscordBot (<url>, <version>)` user agent Discord asks for, and time out after 10 seconds.

A response that isn't a success becomes `Error::Api` with the status and Discord's own error code and message. `Error::status` gives the status, for example to tell a 403 (missing permissions) from a 404 (unknown guild or channel).

### Rate limits

Discord limits each route separately, plus a global limit per bot. The client follows its headers:

- **Per route:** after each response it records `X-RateLimit-Remaining` and `X-RateLimit-Reset-After`. When a route has no calls left, the next call to it waits until the reset.
- **Routes:** a route is the method and path, with IDs replaced except the channel, guild or webhook ID Discord limits separately. So `GET /guilds/1/members/2` and `GET /guilds/1/members/3` share a limit, and `GET /guilds/1` and `GET /guilds/2` don't.
- **429s:** the client waits the `retry_after` from the response body, or the `Retry-After` header, then retries, up to `RateLimitConfig::max_retries` times (3). A global 429 pauses every route.
- **Long waits:** a call that would wait longer than `RateLimitConfig::max_wait` (30 seconds) fails at once with `Error::RateLimited`, so a visitor's request is never held for long.

Two calls made at once can both see one remaining call; the second gets a 429 and is retried. Every response is counted in `discord_requests_total`, labelled by `status`.

## Interactions endpoint

`router` serves `POST /discord/interactions`, the URL to set as the **Interactions Endpoint URL** in the Discord Developer Portal.

Every request must be signed by Discord. The endpoint checks the `X-Signature-Ed25519` header against the application's public key, over the `X-Signature-Timestamp` header followed by the raw body:

- A missing or wrong signature gets a 401. Without this check, anyone could send fake button clicks.
- A timestamp more than 5 minutes from the server's clock (`MAX_TIMESTAMP_SKEW`) gets a 401 too, so a captured request can't be replayed later.
- A signed body that isn't an interaction gets a 400.

Discord sends a signed PING, and a badly signed request, when the URL is saved in the portal; the endpoint answers the PING with a PONG and rejects the other, as Discord requires. Interactions this crate doesn't handle yet get a private "This action isn't available yet." reply.

## Sign in with Discord

`router` also serves the OAuth2 sign-in. Add `<PUBLIC_URL>/api/v1/auth/discord/callback` as a Redirect under OAuth2 in the Discord Developer Portal.

| Route | Does |
|---|---|
| `GET /api/v1/auth/discord/login?flow=leader&next=/path` | Redirects to Discord's authorize page. `next` is where the visitor lands afterwards: a path on this site, else `/` |
| `GET /api/v1/auth/discord/callback` | Where Discord sends the visitor back. Checks the `state`, trades the code for the visitor's Discord ID, starts the session and redirects to `next` |
| `GET /api/v1/leaders/me` | `{"discord_id": "…"}` for a signed-in leader, else 401. Never cached |
| `POST /api/v1/auth/sign-out` | Clears the session cookie (204). A POST, so another site can't trigger it with a link |

**Flows.** The scopes depend on what the sign-in is for: `leader` asks for `identify`, `opt_in` for `bot identify` and `apply` for `identify guilds.join`. Only `leader` is open so far; the others answer 404 until the features that use them land.

**State and nonce.** `login` sends Discord a `state` signed with `TOKEN_SIGNING_KEY` (HMAC-SHA256): the flow, where to go next, a random nonce and a 10-minute expiry (`MAX_STATE_AGE`). The same nonce goes to the browser in an `HttpOnly`, `Secure`, `SameSite=Lax` cookie that only the callback path receives. The callback needs both, and the `state` must verify and not have expired, so a forged, tampered, expired or someone else's callback gets a 400 before Discord is called. The nonce cookie is cleared by every callback, so a replay from the same browser carries none. Discord's own codes work once.

**Session.** A signed cookie holds the Discord ID and a 12-hour expiry (`MAX_SESSION_AGE`), also `HttpOnly`, `Secure` and `SameSite=Lax`. Nothing is stored server-side, so signing out removes the cookie but can't revoke a copy of it before it expires. The signed values carry a purpose, so a `state` can't be used as a session, and neither can the human check's tokens that share the key. `Secure` is left off only when `PUBLIC_URL` is `http://`, for local development.

**What's kept.** The access token is used for one `GET /users/@me` inside the callback request and then dropped; only the ID is kept, inside the session cookie. Nothing from Discord is logged: a failed sign-in logs Discord's error message, not the user. Results are counted in `discord_sign_ins_total`, labelled by `result` (`success`, `invalid_state`, `denied` or `discord_error`).
