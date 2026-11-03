# uma-stable-match-discord

The Discord side of club applications: a REST client that calls Discord as the bot, and the endpoint Discord sends interactions to, such as button clicks. The server mounts it when Discord is configured; see [Discord](../server/README.md#discord-applications) in the server README.

There's no gateway connection and no always-on bot process. Everything the bot does is a REST call, and everything Discord tells it arrives as a signed HTTP request.

## Usage

```rust,no_run
use serde_json::Value;
use uma_stable_match_discord::{
    router, ApplicationId, BotToken, ClientSecret, Discord, DiscordConfig, PublicKey,
};

# async fn run() -> Result<(), uma_stable_match_discord::Error> {
// Read these from your environment; never hard-code them.
let env = |name| std::env::var(name).unwrap_or_default();
let config = DiscordConfig::new(
    ApplicationId::parse(&env("DISCORD_APP_ID")).expect("a valid application ID"),
    PublicKey::parse(&env("DISCORD_PUBLIC_KEY")).expect("a valid public key"),
    BotToken::parse(&env("DISCORD_BOT_TOKEN")).expect("a bot token"),
    ClientSecret::parse(&env("DISCORD_CLIENT_SECRET")).expect("a client secret"),
);
let discord = Discord::new(config)?;

// REST calls as the bot.
let guild: Value = discord.rest().get("/guilds/123").await?;

// POST /discord/interactions, to merge into an axum app.
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
