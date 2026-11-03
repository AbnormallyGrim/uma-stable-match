# uma-stable-match-discord

The Discord side of club applications: a REST client that calls Discord as the bot, the endpoint Discord sends interactions to, such as button clicks, Sign in with Discord, and the club-to-server links kept in Redis. The server mounts it when Discord is configured; see [Discord](../server/README.md#discord-applications) in the server README.

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

**What's kept.** The access token is used for one `GET /users/@me` inside the callback request and then dropped; only the ID is kept, inside the session cookie. Nothing from Discord is logged: a failed sign-in logs Discord's error message, not the user. Results are counted in `discord_sign_ins_total`, labelled by `result` (`success`, `invalid_state`, `denied`, `discord_error`, or for opting a club in `no_server`, `wrong_server` and `unavailable`).

## Finding a club's server

A club points to its Discord server with an invite in its in-game description. `invite_candidates(description)` pulls out the codes to try, at most 5 (`MAX_INVITE_CANDIDATES`), in the order they appear and without repeats:

- **Links:** `discord.gg/…`, `discord.com/invite/…` and `discordapp.com/invite/…`, with or without `https://` and `www.`, in any case.
- **Bare codes:** a code after the word "discord", such as `Discord: aB3dEfG` or `Discord invite - 8xk2mQ`. To keep ordinary words out (`Discord server`), it must be 6 to 12 characters and have a digit or capitals inside it.

These are only guesses. A code is an invite once Discord resolves it: `resolve_invite(rest, code)` calls `GET /invites/{code}` and gives the server's ID. Discord's `404 Unknown Invite` (expired, deleted or never real) and an invite that isn't for a server give `None`; any other failure, such as an outage or a rate limit that runs out, is an `Error`, so it's never read as a missing invite.

`servers_in_description(rest, description)` does both and returns the distinct server IDs, in order. Servers are compared by ID, never by link text, so any working invite to a server counts. An empty list means the club points to no server. If invites point to different servers, all are returned and the caller decides.

## Club links

`ClubStore` keeps which club is linked to which server, in Redis, through a `redis::aio::ConnectionManager`. Neither key has a TTL, so `volatile-lru` eviction can't remove them.

| Key | Holds |
|---|---|
| `discord:club:{club_id}` | A hash: `server`, `category`, `leader_roles` (comma-separated), `auto_join` and `open` (`1` or absent), and `problem` |
| `discord:guild:{guild_id}` | A set of the IDs of the clubs linked to that server |

Many clubs can share a server, and a club is linked to one server at a time:

- `link(club_id, server_id)` links a club. Linking it to another server moves it: it leaves the old server's set and loses its `category`, `leader_roles` and `problem`, which name things on the old one, and keeps its `auto_join` and `open` choices. It returns the `Departure` (server and clubs still linked there) if the club moved.
- `unlink(club_id)` deletes the club's hash and takes it out of its server's set, returning the `Departure` if it was linked. A `remaining` of 0 means no club needs the bot on that server any more.
- `club(club_id)` reads the `ClubRecord`, and `clubs_on(server_id)` lists a server's clubs, lowest ID first.

Each of `link` and `unlink` is one Lua script, so a crash can't leave a club in one set and not the other. Calls time out after 5 seconds. Errors name only the kind of Redis failure, as Redis's own messages can include its URL.

## Opting a club in

A leader opts a club in by showing they control the Discord server the club points to. Two things are checked, and Discord vouches for both:

- **The club points to the server:** its description has a working invite to it (see [Finding a club's server](#finding-a-clubs-server)).
- **The person controls the server:** they add the bot to that same server, which Discord only allows with Manage Server or Administrator.

`GET /api/v1/discord/opt-in?club=ID&next=/path` starts it. It needs a leader session (sign in first), so a visitor can't make the server call Discord for nothing. It answers:

| Status | When |
|---|---|
| 401 | Not signed in |
| 400 | `club` isn't a club ID |
| 404 | No such club |
| 422 | The description has no working invite, or invites to more than one server |
| 502 | The club or Discord couldn't be reached; try again |
| 307 | To Discord's add-bot screen (`scope=bot identify`) with `guild_id` set and the server picker locked |

The club, the expected server and the signed-in user go inside the signed `state`, so editing the callback's query string can't change them. `permissions` is `BOT_PERMISSIONS`: View Channels, Manage Channels, Manage Roles, Send Messages, Embed Links, Read Message History and Create Instant Invite. No `guilds` scope is asked for: adding the bot already proves the person's rights, and their server list isn't needed.

The callback never reads the redirect's own `guild_id` or `permissions` parameters. It exchanges the code, and Discord's token response says which server the bot was actually added to:

- **Same server as the invite's, and the same person who started it:** the club is linked (`ClubStore::link`) and the visitor goes to `next`. If the club was linked to another server, it moves; when no club is left on the old one, the bot leaves it.
- **Another server:** nothing is linked (403) and the bot leaves that server, unless a club is linked to it.
- **No server** (the bot wasn't added) **or a different person:** nothing is linked (400).

Opting in needs the app set up with `Discord::with_opt_in(OptIn::new(store, clubs))`. `ClubLookup` is how clubs are found, since this crate doesn't know where they come from; it answers `Ok(None)` for a club that doesn't exist. Without it, the route isn't served.

## Unlinking a club

`DELETE /api/v1/discord/clubs/{club}` (`UNLINK_PATH`) takes a club off its server, deleting everything kept about it. It needs a leader session, and the person must have the rights that let them add the bot: they own the server or have Administrator or Manage Server there. The bot checks this itself with two calls, `GET /guilds/{id}` (the owner and the roles' permissions) and `GET /guilds/{id}/members/{user}` (the person's roles), and combines the permissions of `@everyone` and the person's roles.

| Status | When |
|---|---|
| 204 | The club is unlinked |
| 401 | Not signed in |
| 400 | `club` isn't a club ID |
| 404 | The club isn't linked to a server |
| 403 | The person isn't a member with those rights, or the bot can't see the server any more, so nothing proves their rights |
| 502 | Discord couldn't be reached to check; try again |
| 503 | The saved links couldn't be reached |

It is a `DELETE`, which a browser doesn't send across sites with the session cookie. When the last club on a server is unlinked the bot leaves it; while another club is linked there the bot stays.

A club whose server has removed the bot can't prove anyone's rights, so it can't be unlinked this way. Opting the club in again, to the same or another server, replaces the link.
