# uma-stable-match-discord

The Discord side of club applications: a REST client that calls Discord as the bot, the endpoint Discord sends interactions to, such as button clicks, Sign in with Discord, and the club-to-server links kept in Redis. The server mounts it when Discord is configured; see [Discord](../server/README.md#discord-applications) in the server README.

Everything the bot does is a REST call, and everything Discord tells it arrives as a signed HTTP request. The one exception is the [gateway connection](#gateway), which only makes the bot show as online.

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

## Gateway

`Discord::run_gateway` opens the bot's gateway connection (`twilight-gateway`, no privileged intents) so Discord shows the bot online. It receives no events and sends nothing beyond the heartbeat, a few bytes every 40 seconds or so. It reconnects and resumes by itself, and returns only when Discord closes it for good, such as for a bad token, after which the bot shows offline.

The server spawns it once at startup. Run one instance of the server: each one holds its own connection.

## Interactions endpoint

`router` serves `POST /discord/interactions`, the URL to set as the **Interactions Endpoint URL** in the Discord Developer Portal.

Every request must be signed by Discord. The endpoint checks the `X-Signature-Ed25519` header against the application's public key, over the `X-Signature-Timestamp` header followed by the raw body:

- A missing or wrong signature gets a 401. Without this check, anyone could send fake button clicks.
- A timestamp more than 5 minutes from the server's clock (`MAX_TIMESTAMP_SKEW`) gets a 401 too, so a captured request can't be replayed later.
- A signed body that isn't an interaction gets a 400.

Discord sends a signed PING, and a badly signed request, when the URL is saved in the portal; the endpoint answers the PING with a PONG and rejects the other, as Discord requires. Clicks on a ticket's Accept and Decline buttons are [answered](#applying-to-a-club). Other interactions get a private "This action isn't available yet." reply.

## Webhook events

`router` also serves `POST /discord/webhook-events` (`WEBHOOK_EVENTS_PATH`), the endpoint to set under **Webhooks** in the Discord Developer Portal, with **Events** on and **Application Deauthorized** picked.

- Requests are signed like interactions and checked the same way: a missing or wrong signature, or a timestamp more than 5 minutes off, gets a 401. Discord sends badly signed requests on purpose and removes the URL if one is accepted.
- A signed PING (`type` 0) gets an empty 204, which Discord needs before it saves the URL.
- `APPLICATION_DEAUTHORIZED`, sent when someone removes the app from their Discord account, ends every session of `event.data.user` on any device, through the `SessionStore`, and drops their cached name (204). If Redis can't be reached it answers 503, and Discord retries for up to 10 minutes. Trainer links stay, as they belong to the site, not to Discord. Counted in `discord_deauthorizations_total`.
- Other events get a 204 and are ignored.

## Sign in with Discord

`router` also serves the OAuth2 sign-in. Add `<PUBLIC_URL>/api/v1/auth/discord/callback` as a Redirect under OAuth2 in the Discord Developer Portal.

| Route | Does |
|---|---|
| `GET /api/v1/auth/discord/login?flow=sign_in&next=/path` | Redirects to Discord's authorize page. `next` is where the visitor lands afterwards: a path on this site, else `/` |
| `GET /api/v1/auth/discord/callback` | Where Discord sends the visitor back. Checks the `state`, trades the code for the visitor's Discord ID, starts the session and redirects to `next`. When it fails, a browser (a request that accepts `text/html`) is redirected to `/applications?error=<result>` (`APPLICATIONS_PAGE`) instead of seeing JSON |
| `GET /api/v1/auth/session` | `{"discord_id": "…", "name": "…"}` for a signed-in person, else 401. `name` is their Discord display name, or their username without one, and `null` if Discord couldn't be asked. Never cached |
| `POST /api/v1/auth/sign-out` | Ends the session and clears its cookie (204), so a copy of the cookie stops working too. 503 if that can't be saved (the session stays). A POST, so another site can't trigger it with a link, and refused (403) when its `Origin` isn't this site |
| `DELETE /api/v1/auth/account` | Deletes the person's [trainer links](#club-links), pending keys and attempt counts, ends every session they have, on any device, and clears the cookie (204). Clubs they opted in stay linked. 401 without a session, 503 if the saved links can't be reached (the session stays; try again). A DELETE, so another site can't trigger it with a link or form (`ACCOUNT_PATH`) |

**Flows.** The scopes depend on what the sign-in is for: `sign_in` asks for `identify`, `opt_in` for `bot identify` and `apply` for `identify guilds.join`. `login` only starts `sign_in`; opting a club in and applying start from their own routes, which set the flow and its target.

**State and nonce.** `login` sends Discord a `state` signed with `TOKEN_SIGNING_KEY` (HMAC-SHA256): the flow, where to go next, a random nonce and a 10-minute expiry (`MAX_STATE_AGE`). The same nonce goes to the browser in an `HttpOnly`, `Secure`, `SameSite=Lax` cookie that only the callback path receives. The callback needs both, and the `state` must verify and not have expired, so a forged, tampered, expired or someone else's callback gets a 400 before Discord is called. The nonce cookie is cleared by every callback, so a replay from the same browser carries none. Discord's own codes work once.

**Session.** A signed cookie holds the Discord ID and a 12-hour expiry (`MAX_SESSION_AGE`), also `HttpOnly`, `Secure` and `SameSite=Lax`. Signing out and deleting the account end sessions early; see **Ending sessions** below. The signed values carry a purpose, so a `state` can't be used as a session, and neither can the human check's tokens that share the key. `Secure` is left off only when `PUBLIC_URL` is `http://`, for local development.

**Ending sessions.** With a `SessionStore` (`Discord::with_sessions`), every signed-in request also checks Redis, so a session can end before its 12 hours are up. A Redis failure answers 503 rather than treating the person as signed out. Without one, signing out only clears the cookie.

| Key | Holds |
|---|---|
| `discord:session:revoked:{hash}` | A session that signed out, by a hash of its cookie, until it would have expired. Only that browser's session ends; others stay signed in |
| `discord:session:ended:{hash}` | When the person, by the keyed hash of their Discord ID, had every session ended, for 12 hours. Deleting the account and removing the app from one's Discord account ([webhook events](#webhook-events)) set it, and sessions started before then stop working. One started in the same second is kept, so signing in again at once works |

**Sign-out's origin.** Browsers send `Origin` with every `POST`. Sign-out is refused when it names another site, so a form elsewhere can't sign people out. The scheme and host must match `PUBLIC_URL`; the port is ignored, so a local dev server's proxy still works. A request with no `Origin` isn't from a browser page and is allowed.

**What's kept.** The access token is used for one `GET /users/@me` inside the callback request and then dropped; only the ID is kept, inside the session cookie. Beside it, the page-readable `uma_signed_in` cookie holds only the session's expiry, so the web app knows a session exists without asking; it proves nothing. Signing in sets it, signing out, deleting the account and a `401` from `GET /api/v1/auth/session` clear it, and a successful session check puts it back if it's missing. The name `GET /api/v1/auth/session` gives is looked up as the bot (`GET /users/{id}`) and held in memory for 10 minutes (`NAME_TTL`), never in the cookie or Redis; deleting the account drops it. Nothing from Discord is logged: a failed sign-in logs Discord's error message, not the user. Results are counted in `discord_sign_ins_total`, labelled by `result` (`success`, `invalid_state`, `denied`, `discord_error`, or for opting a club in `no_server`, `wrong_server`, `not_manager`, `not_leader`, `missing_permissions` and `unavailable`).

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
| `discord:club:{club_id}` | A hash: `server`, `category`, `auto_join` and `open` (`1` or absent), `leader` (the Discord ID of the leader who last opted the club in or changed its settings, who is added to its tickets; encrypted, see below), `ticket_prefix` or `ticket_name` (at most one, see [The ticket name](#the-ticket-name)) and `problem`. Clubs linked before leader roles were dropped may still hold `leader_roles`, which nothing reads |
| `discord:guild:{guild_id}` | A set of the IDs of the clubs linked to that server |

Many clubs can share a server, and a club is linked to one server at a time:

- `link(club_id, server_id)` links a club. Linking it to another server moves it: it leaves the old server's set and loses its `category` and `problem`, which name things on the old one, and keeps its `auto_join`, `open` and ticket name choices. It returns the `Departure` (server and clubs still linked there) if the club moved.
- A new link starts open and with auto-join on (`open` and `auto_join` are set only if the club has none, so a moved club keeps its choices).
- `update(club_id, server_id, &SettingsUpdate)` changes the category, auto-join, open or ticket name fields, and only while the club is still linked to `server_id`, so a club that moved meanwhile isn't changed. It returns whether it did.
- `takes_applications(club_ids)` says for each club whether it is linked, open and has no setup problem, in one round trip. The server adds the club's own join style and spots to decide the `applications` badge.
- `unlink(club_id)` deletes the club's hash and takes it out of its server's set, returning the `Departure` if it was linked. A `remaining` of 0 means no club needs the bot on that server any more.
- `club(club_id)` reads the `ClubRecord`, and `clubs_on(server_id)` lists a server's clubs, lowest ID first.
- `with_dev_guild(server_id)` makes the store answer as if every club were open in that server, for trying applications on one's own machine. Only the ticket name is still read from Redis, so a leader's choice shows. The server only allows it on a `localhost` address.

The leader's Discord ID is encrypted like an [applicant's](#applying-to-a-club), with the club's ID as associated data, so it can't be copied onto another club. `ClubStore::new` takes the key secret (`REDIS_KEY_SECRET`) for it. A leader saved as a plain number before this is still read, and is encrypted the next time the club is opted in or its settings change.

Each of `link` and `unlink` is one Lua script, so a crash can't leave a club in one set and not the other. Calls time out after 5 seconds. Errors name only the kind of Redis failure, as Redis's own messages can include its URL.

## Opting a club in

Only a club's leader can add the bot. Four things are checked:

- **The person leads the club:** the trainer ID they [linked](#linking-a-trainer) is the club's leader in the game. See [Leader access](#leader-access).
- **The club points to the server:** its description has a working invite to exactly one server (see [Finding a club's server](#finding-a-clubs-server)). Only `discord.gg/`, `discord.com/invite/` and `discordapp.com/invite/` links and bare codes after the word "discord" count, and each code is checked with Discord, so a link to any other site, an expired invite or one that leads to no server never counts.
- **The person manages the server:** Discord only lets someone with Manage Server or Administrator add a bot, and the bot checks it again before linking.
- **The bot went to that server:** Discord's token response names the server it was added to.

Many clubs can share a server; each club's leader adds it for their own club.

`GET /api/v1/discord/opt-in?club=ID&next=/path` starts it. It needs a session (sign in first), so a visitor can't make the server call Discord for nothing. The leader is checked before the invite, so someone else learns nothing about it. It answers:

| Status | When |
|---|---|
| 401 | Not signed in |
| 400 | `club` isn't a club ID |
| 403 | No trainer linked, or the linked trainer doesn't lead the club |
| 404 | No such club |
| 422 | The description has no working invite, or invites to more than one server |
| 502 | The club or Discord couldn't be reached; try again |
| 307 | To Discord's add-bot screen (`scope=bot identify`) with `guild_id` set and the server picker locked |

The club, the expected server and the signed-in user go inside the signed `state`, so editing the callback's query string can't change them. `permissions` is `BOT_PERMISSIONS`: View Channels, Manage Channels, Manage Roles, Send Messages, Embed Links, Read Message History and Create Instant Invite. No `guilds` scope is asked for: the person's server list isn't needed.

The callback never reads the redirect's own `guild_id` or `permissions` parameters. It exchanges the code, and Discord's token response says which server the bot was actually added to:

- **Same server as the invite's, the same person who started it, who manages it and still leads the club:** the club is linked (`ClubStore::link`) and the visitor goes to `next`. If the club was linked to another server, it moves; when no club is left on the old one, the bot leaves it. Managing the server is checked with `GET /guilds/{id}` (the owner and the roles' permissions) and `GET /guilds/{id}/members/{user}` (the person's roles), combining `@everyone`'s permissions and the person's roles.
- **Another server, or a person who doesn't manage it or no longer leads the club:** nothing is linked (403) and the bot leaves that server, unless a club is linked to it.
- **The bot without every permission in `BOT_PERMISSIONS`:** Discord's page lets the person untick permissions, so the bot's own roles are checked (`GET /guilds/{id}` and `GET /guilds/{id}/members/{application id}`, combining `@everyone` and the bot's roles; Administrator covers everything, channel overrides aren't looked at). If any is missing, nothing is linked (400, `missing_permissions`) and the bot leaves, unless a club is linked to it. The person starts again and leaves them ticked.
- **No server** (the bot wasn't added) **or a different person:** nothing is linked (400).

Opting in needs the app set up with `Discord::with_opt_in(OptIn::new(store, clubs))` and `Discord::with_trainer_links`. `ClubLookup` is how clubs are found, since this crate doesn't know where they come from; it answers `Ok(None)` for a club that doesn't exist, and the server reads the club fresh from uma.moe each time. Without both, the routes aren't served.

## Leader access

Everything about a linked club, from its settings to unlinking it, is for its leader alone. On every call, the club is read fresh (`ClubLookup`) and its leader's trainer ID must be one the signed-in person has linked, so a leader who hands the club over loses access at once and the new leader gains it. Managing the club's Discord server isn't enough, and isn't needed after the bot is in.

| Status | When |
|---|---|
| 401 | Not signed in |
| 400 | `club` isn't a club ID |
| 403 | No trainer linked, or the linked trainer doesn't lead the club |
| 404 | No such club, or it isn't linked to a server |
| 502 | The club couldn't be looked up; try again |
| 503 | The saved links couldn't be reached |

`GET /api/v1/discord/my-club` (`MY_CLUB_PATH`) says where the signed-in person stands, so the page can find their club without being told it:

```json
{ "trainer": "100000001", "club": { "id": "900001", "name": "Example Club", "leader": true } }
```

`trainer` is their linked trainer (`null` with none), and `club` the club it's in, from its profile (`null` with none), with whether it leads it. It's never cached. Someone who linked several trainers before only one was allowed gets the first that leads a club. It needs a session (401), and a 502 means uma.moe couldn't be reached.

## Applying to a club

`GET /api/v1/discord/apply?club={id}` (`APPLY_PATH`) is where a club's Quick Apply button goes. Applying needs `Applying`, set with `Discord::with_applications`: an `ApplicationStore` and an `ApplicantLookup`, which gives a trainer's name, club, pace, fans this month and all-time fans.

1. Not signed in: the player is sent through Discord's sign-in and back to the same address.
2. Signed in: the club must be linked, open and without a setup problem, and the player must have a linked trainer. If not, a browser lands on `/applications?error=<label>` (`not_open`, `no_trainer`, `bad_club`) and anything else gets JSON. Otherwise the player goes to Discord with the `identify guilds.join` scopes.
3. The callback finishes it, only for the person who started it. It checks the club again and:
   - saves the application, unless the player has a pending one to this club (`already_applied`, 409) or sent `MAX_PER_DAY` (5) in the last day (`too_many_applications`, 429);
   - adds the player to the club's server with their access token, which lives only in that call. Players are always added, whatever the club's `auto_join` says;
   - opens the ticket and notes it on the application. If any step after saving fails, the application is taken back, so a failed try costs nothing (`join_failed`, `ticket_failed`).
4. The player lands on `/applications/trainer`.

**The ticket** is a private text channel, named as [the club chose](#the-ticket-name), in the club's category if it has one. Everyone else is shut out; the applicant, the leader noted on the club and the bot can see it. If Discord refuses it, the leader may have left the server, so it's tried once more without them. The bot posts an embed with the trainer's name and a link to `https://uma.moe/profile/<trainer ID>`, the applicant, their current club, pace, fans this month, total fans, average fans per day, week and month, fans rank and days tracked, and Accept and Decline buttons.

**The buttons** go through the interactions endpoint. Only the leader noted on the club can answer, or, for a club with no leader noted, someone with Manage Server. The first answer wins and removes the buttons; the applicant can then apply to that club again.

**Lists.** `GET /api/v1/discord/applications` (`APPLICATIONS_PATH`) gives the signed-in player's applications and `GET /api/v1/discord/clubs/{club}/applications` (`CLUB_APPLICATIONS_PATH`) a club's, for its leader (the 100 newest). Both are newest first, never cached, and use the shapes the web app reads.

**What's kept.** In Redis, until a calendar month after it was sent (one calendar month later, on the month's last day when it has no such date):

| Key | Holds |
|---|---|
| `discord:app:{id}` | A hash: the club and its name, server, the applicant's Discord ID (encrypted) and keyed hash, the trainer and its name, current club, pace, fans, status, when it was sent, and the ticket's channel and message |
| `discord:club-apps:{club_id}` | The club's application IDs, scored by when they were sent |
| `discord:my-apps:{hash}` | The same for one applicant |
| `discord:applied:{hash}:{club_id}` | The applicant's pending application to the club, which stops a second one |
| `discord:apply-day:{hash}` | How many applications the applicant sent in the last day, for 24 hours |

The applicant's Discord ID is the only field kept encrypted: AES-256-GCM, with a key derived from the key secret (`REDIS_KEY_SECRET`) and the application's ID as associated data, so a copy moved onto another application doesn't open. Everything else is what uma.moe shows publicly or an ID that is useless alone. Changing the key secret makes saved applications unreadable until they expire; an application that can't be read is skipped in lists, not removed.

The lists drop expired applications when read. Results are counted in `discord_applications_total` by `result`, and answers in `discord_application_answers_total`.

## Ads for clubs

Trainers post an ad that they're looking for a club, and leaders find them on a board and invite them to apply. Ads need an `AdStore` on `Applying`, set with `Applying::with_ads`.

- `GET`, `PUT` and `DELETE /api/v1/discord/ads/me` (`MY_AD_PATH`) read, post or replace, and delete the signed-in trainer's ad. `GET` answers `null` without one. A person has one ad, for their linked trainer; without one `PUT` is 403.
- `POST /api/v1/discord/ads/me/renew` (`MY_AD_RENEW_PATH`) keeps the ad's choices and takes fresh fan numbers.
- `GET /api/v1/discord/ads` (`ADS_PATH`) gives one page of the ads on the board for anyone signed in: `{ "ads": [...], "page", "pages", "total" }`, `PAGE_SIZE` (20) ads a page, fastest first and those without a pace last. Query parameters describe the leader's club. Each is optional, and one left out (or `language=any`) filters nothing:
  - `rank_min` and `rank_max`: tier codes 1 (D) to 11 (SS); swapped if both are given the wrong way round. An ad matches when its lowest rank is at or above `rank_min` and its highest at or below `rank_max`.
  - `has_discord`: `true` or `false`. Without a Discord server (`false`), ads that require one are left out.
  - `language`: a language code or `any`. Ads in another language are left out; ads that don't mind stay.
  - `page`: from 1; a page past the end gives the last one.
  - `club`: a club the signed-in person leads (403 otherwise), so each ad also says whether that club already invited the trainer.

  Anything that can't be read is a 400. The server reads at most `MAX_ON_BOARD` (500) ads, the most recently renewed, then filters, sorts and pages them. Nothing identifies the trainer beyond their name and trainer ID: Discord usernames are never shown.
- `POST /api/v1/discord/clubs/{club}/ads/{ad}/invite` (`CLUB_AD_INVITE_PATH`) invites the trainer. It needs the club's leader and a club that takes applications through Discord (409 otherwise), and answers 204, also for a repeat. A trainer with `MAX_INVITES` (10) is 409, and an ad that's off the board 404.
- `DELETE /api/v1/discord/ads/me/invites/{club}` (`MY_AD_INVITE_PATH`) dismisses an invite.

An ad holds the trainer's name, current club, pace and fans this month, taken from the same `ApplicantLookup` as applications, and what they ask for: the lowest and highest club rank (tier codes 1 for D to 11 for SS), at most three ranks apart counting both (`MAX_RANK_SPAN`: C to B covers C, C+ and B; a wider range is a 400), whether the club needs a Discord server (`required`, `preferred` or `any`) and the language it speaks: `en`, `ja`, `ko`, `zh`, `es`, `pt`, `fr`, `de`, `id`, `th`, `vi` or `any`. The languages are only filters for leaders; the app isn't translated into them. Every ad takes invites, and there is no free text.

An invite is not an addition to a server. The trainer sees it on their ad page and applies through the usual [application](#applying-to-a-club), which signs them in again, adds them and opens the ticket. Nothing from the sign-in is kept for later.

**What's kept.** In Redis:

| Key | Holds |
|---|---|
| `discord:ad:{hash}` | A hash: the ad's fields, the trainer's Discord ID (encrypted as for applications), when it was posted and when it leaves the board, and one `inv:{club_id}` field per invite, holding when it was sent |
| `discord:ads` | The hashes of the ads on the board, scored by when each leaves it |

`{hash}` is the keyed hash of the trainer's Discord ID. An ad is on the board for 7 days (`VISIBLE_FOR`) after it's posted or renewed and kept for 30 (`STORED_FOR`), so a trainer can bring it back; invites live on the ad and go with it. Leaving the board changes nothing in the hash: `board` drops the ad from `discord:ads` when it reads it, and renewing adds it back. Deleting the account or unlinking the ad's trainer deletes the ad.

Invites are one field each, not a list in one field, so the hash stays in Redis's compact encoding (which needs every value under `hash-max-listpack-value`, 64 bytes by default) and an ad with all 10 invites takes under 1 KB. Results are counted in `discord_ads_total` by `result` and invites in `discord_ad_invites_total`.

## Unlinking a club

`DELETE /api/v1/discord/clubs/{club}` (`UNLINK_PATH`) takes a club off its server, deleting everything kept about it. It needs the club's leader (see [Leader access](#leader-access)), and answers 204 once it's unlinked. Discord isn't asked, so a club whose server removed the bot can still be unlinked.

It is a `DELETE`, which a browser doesn't send across sites with the session cookie. When the last club on a server is unlinked the bot leaves it; while another club is linked there the bot stays. Opting the club in again, to the same or another server, replaces the link.

## Club settings

`GET` and `PATCH /api/v1/discord/clubs/{club}/settings` (`SETTINGS_PATH`) read and change a club's settings. Both need the club's leader (see [Leader access](#leader-access)).

`GET` answers with the settings and the categories to choose from. IDs are strings, as Discord's are larger than JSON numbers can hold safely:

| Field | Meaning |
|---|---|
| `guild_id` | The linked server |
| `category` | The category for the club's ticket channels, or `null` |
| `auto_join` | Whether applicants are added to the server automatically |
| `open` | Whether the club takes applications (`false` is paused) |
| `problem` | The club's setup problem, if any; it can't be changed here |
| `ticket_name` | How tickets are named: `{mode, value}`, with `mode` one of `default`, `prefix` or `override` and `value` the text, `null` for `default` |
| `categories` | The server's categories, `{id, name}`, to choose the ticket category from |

`PATCH` takes a JSON object with any of `category` (an ID, or `null` to clear), `auto_join`, `open` and `ticket_name` (`{mode, value}` as above). Fields left out stay as they are. The category must be one of the server's categories and the ticket name must pass [its checks](#the-ticket-name); otherwise, an unknown field, a wrong type, or no field at all gets 400 and nothing changes. It answers with the new settings, as `GET` does. A 409 means the club moved to another server or was unlinked while the change was made.

**Bot kicked from the server.** Discord then answers the channel list with 403 `Missing Access` (50001) or 404 `Unknown Guild` (10004). Both `GET` and `PATCH` take that as the bot being gone: every club linked to that server is unlinked (the bot is gone for all of them) and the call answers 404, as for a club that isn't linked, so the leader can opt in again. Each is counted in `discord_unlinks_total` with `result="bot_removed"`. Any other Discord failure is 502 and changes nothing.

### The ticket name

A leader picks how the club's tickets are named. Every name ends in a dash and a random number from `0000` to `9999`, which tells tickets apart; Discord doesn't need names to be unique.

| `mode` | Channel name |
|---|---|
| `default` (nothing stored) | `<club>-<trainer>-<number>` |
| `prefix` | `<prefix>-<trainer>-<number>` |
| `override` | `<override>-<number>` |

The club and trainer names come from uma.moe, so they are cleaned up rather than refused: lowercase letters and digits joined by dashes. A trainer with none of those is `applicant`, and a club with none is left out. When a name would pass 90 characters, the club or prefix is cut, then the trainer's name (at most 40), so the number always fits.

A prefix or override comes from the leader, so it is refused, never repaired. Only the server's checks count; the dashboard's copy is for quick feedback:

1. More than 80 bytes is refused before anything else is looked at.
2. The text is trimmed, ASCII letters are lowercased and each space or underscore becomes a dash.
3. What's left must be letters `a-z` and digits joined by single dashes. Anything else, such as quotes, brackets, `@`, `#`, dots, slashes, control characters, zero-width or right-to-left characters, look-alike letters and emoji, is refused.
4. A prefix may be 32 characters and an override 80.

A bad value gets 400 and nothing changes. The checked, lowercased text is what's stored, in `ticket_prefix` or `ticket_name`, one at a time: saving one empties the other, and `default` empties both. It is passed to Redis as an argument, never built into the script or a key, and to Discord as a JSON value. It is checked again when read, so a value put in Redis some other way names no channel and the default is used. Tickets already open keep the name they have.

## Linking a trainer

A trainer ID is public and typed in by the player, so it proves nothing by itself. A signed-in person proves they own one by putting a key we give them in their in-game profile comment; once uma.moe shows the key there, the ID is linked to their Discord ID.

| Route | Does |
|---|---|
| `GET /api/v1/discord/trainers` | The trainers the signed-in person owns, the most they can, and their pending keys, newest first (`TRAINERS_PATH`); see below |
| `POST /api/v1/discord/trainers/{trainer}/verification` | Answers `{"key": "uma-7KQ4XMPR", "expires_in": 86400}`: the key to put in the profile comment (`VERIFICATION_PATH`) |
| `DELETE /api/v1/discord/trainers/{trainer}/verification` | Forgets the pending key and stops its check (204) |
| `POST /api/v1/discord/trainers/{trainer}/link` | Looks for the key in the trainer's profile and links the ID if it's there; otherwise keeps looking in the background (`LINK_PATH`) |
| `DELETE /api/v1/discord/trainers/{trainer}` | Unlinks the ID from the signed-in person (`TRAINER_PATH`) |

All need a session, and `{trainer}` is 9 to 12 digits. The key and the list are never cached (`Cache-Control: no-store`).

**Keys.** A key is `uma-` and eight characters from letters and digits that can't be mistaken for each other (no `I`, `L`, `O`, `0` or `1`): 31⁸, about 850 billion keys. No two pending keys are the same: a key is marked as taken while it is pending, and if a new one is already taken another is made (up to 5 tries, then a 503). The mark goes when the key links, expires, or its owner deletes their account. It is made for one person and one trainer ID, is good for 24 hours (`KEY_LIFETIME`), and is used up when it links. Asking again while one is pending gives the same key. Someone else can't use a key they read in a profile: their own check looks for their own key. The profile can take a while to show a new comment, so the key lasts a day, and checking again is fine.

The list answers:

```json
{
  "linked": [{ "id": "100000000001", "name": "Example Trainer" }],
  "max": 1,
  "pending": [
    {
      "trainer": "100000000002",
      "key": "uma-7KQ4XMPR",
      "expires_in": 86000,
      "check": { "state": "checking", "seconds_left": 240 }
    }
  ]
}
```

`name` is the trainer's name when it linked (`null` for links made before names were kept). `check` is `null` until a check runs; then `checking` with the seconds it has left, or how it ended for 15 minutes after: `missing` (never found), `not_found` (uma.moe doesn't know the trainer) or `too_many` (no room). A check that links leaves no `pending` entry, and the trainer shows under `linked`.

**Checking.** The check looks for the key anywhere in the comment, ignoring case, full-width forms and the kind of dash (the game saves `-` as `－`), in a profile fetched fresh from uma.moe (`TrainerLookup`, given by the server, so this crate doesn't know where profiles come from). It answers:

| Status | When |
|---|---|
| 200 | Linked: `{"linked": true, "name": "…"}` |
| 202 | Not found yet, or uma.moe couldn't be reached: `{"checking": true, "seconds_left": 300}`. The server keeps looking (see below). Asking again while it runs gives the same answer without looking again |
| 400 | `trainer` isn't a trainer ID |
| 401 | Not signed in |
| 403 | The person already has `MAX_LINKED_TRAINERS` (1) ID linked; nothing changes, so no one loses their link to a refused claim |
| 404 | uma.moe doesn't know the trainer |
| 409 | No key is pending (ask for one first, or it expired); uma.moe isn't called |
| 429 | Too many tries; `Retry-After` says how many seconds to wait. At most 10 checks and 20 keys an hour per person, and a refused check never reaches uma.moe. A check with its retries counts once |
| 503 | The saved links couldn't be reached |

Asking for a key for an ID the person already owns is a 409 as well, and asking for one while they have a trainer linked is a 403: they unlink it first.

**Background checks.** When the first look misses, the check keeps going on the server: it looks again every minute (`CHECK_EVERY`) until 5 minutes (`CHECK_FOR`) have passed, six looks in all. Its state is in Redis, so closing or reloading the page doesn't stop it, and the list shows it to a page opened later. `TrainerLinks::run_checks` runs them: the server spawns it once, and every 5 seconds it claims the checks that are due from a sorted set scored by when to look next. Claiming moves a check a minute on first, so several servers sharing Redis never look at once, and checks left by a restart are picked up when the server starts again. A check ends when it links, when the key is forgotten or expires, when uma.moe doesn't know the trainer, when the person has no room, or after the last look.

**Owners.** A trainer ID has one owner at a time. Proving it again replaces the old owner, since the comment shows who controls the account now. A person can own one ID at a time, which is how a club's leader is found.

**What's kept.** Redis never holds a Discord ID, only a keyed hash of it (`KeySecret`, the server's `REDIS_KEY_SECRET`). The trainer ID's owner and the person's set of IDs have no TTL, so eviction can't remove them; they stay until the person unlinks or deletes their account, and so do the trainers' names, kept when they link. The pending key, the check and the attempt counters expire.

| Key | Holds |
|---|---|
| `discord:trainer:{trainer_id}` | The hash of the owner |
| `discord:applicant:{hash}` | A set of the trainer IDs that person owns |
| `discord:names:{hash}` | A hash of those trainers' names, by trainer ID |
| `discord:verify:{hash}:{trainer_id}` | The pending key, for 24 hours |
| `discord:pending:{hash}` | A set of the trainer IDs the person has keys pending for, for 24 hours |
| `discord:key:{key}` | Marks a pending key as taken, for as long as it is pending |
| `discord:check:{hash}:{trainer_id}` | A hash: the check's `state` and `until`, kept 15 minutes after it ends |
| `discord:checks` | A sorted set of running checks (`{hash}:{trainer_id}`), scored by when to look next. `TrainerStore::with_check_queue` names it differently, so apps sharing a Redis (tests) keep apart |
| `discord:hits:{action}:{hash}` | Attempts this hour |

Linking needs the app set up with `Discord::with_trainer_links(TrainerLinks::new(store, lookup))`; without it the routes aren't served. Results are counted in `discord_trainer_links_total`, labelled by `result` (`linked`, `mismatch`, `not_found`, `too_many`, `rate_limited`, `unavailable`, and `gave_up` when a background check ends without the key). Each look counts, the background ones included.
