# Hosting on Fly.io

How to run a public instance on [Fly.io](https://fly.io), with Redis keeping its data across restarts, logs searchable in [Axiom](https://axiom.co) and an alert when the site goes down. [`fly.toml`](../fly.toml) in the repository root holds the server's settings, and [`deploy/redis`](../deploy/redis) holds Redis's.

```text
visitors → Fly proxy (HTTPS) → server machine ─ logs → log shipper machine → Axiom
                                     │  ↑
                                     │  uptime monitor checks /api/v1/health
                                     └─ private network → Redis machine + volume
```

It runs three always-on machines (the server, Redis and the log shipper) and a 1 GB volume. Fly.io needs a payment card; check its pricing for the current cost of small machines and volumes.

## 1. Install the Fly CLI and sign in

```bash
brew install flyctl
```

On other platforms, follow [Fly's install guide](https://fly.io/docs/flyctl/install/). Then run `fly auth signup`, or `fly auth login` if you have an account.

## 2. Set up Axiom

1. Sign up at [axiom.co](https://axiom.co) and pick a region near where the server runs. `fly.toml` uses `iad` (US East), so pick US East 1. The region can't be changed later.
2. Create a dataset, such as `uma-stable-match`.
3. Under Settings → API tokens, create a token that can only ingest into that dataset. You need it in step 6.

## 3. Create the app and set the API key

From the repository root, using the `app` name from `fly.toml` (change it there first if the name is taken):

```bash
fly apps create uma-stable-match
```

Set your uma.moe API key as a secret. Run this, paste `UMAMOE_API_KEY=<your key>`, press Enter, then Ctrl+D. It doesn't go into your shell history:

```bash
fly secrets import --stage
```

`--stage` saves the secret without deploying.

## 4. Set up Redis

Redis keeps club details, the published scan and the refresh schedule across restarts and deploys, so they aren't loaded from uma.moe again; see [Shared cache](../backend/server/README.md#shared-cache). It runs as its own app from [`deploy/redis`](../deploy/redis), with no public address: the server reaches it on your Fly private network.

Create the app and its volume (1 GB, snapshotted daily and kept 5 days). Fly warns that one volume has no redundancy; accept, as the snapshots cover it:

```bash
fly apps create uma-stable-match-redis
```

```bash
fly volumes create redis_data --size 1 --region iad -a uma-stable-match-redis
```

Generate a password and set it on both apps, with a separate secret for hashing trainer IDs. No one needs to see or type either:

```bash
P=$(openssl rand -hex 32) && fly secrets set --stage -a uma-stable-match-redis REDIS_PASSWORD="$P" && fly secrets set --stage -a uma-stable-match REDIS_URL="redis://:$P@uma-stable-match-redis.internal:6379" REDIS_KEY_SECRET="$(openssl rand -hex 32)"; unset P
```

Deploy Redis before the server, which stops if it can't reach Redis at start-up:

```bash
fly deploy deploy/redis --ha=false
```

`fly checks list -a uma-stable-match-redis` shows its check passing. If the server is already running, deploy it again (step 5) to switch it to Redis.

## 5. Deploy

```bash
fly deploy --ha=false
```

`--ha=false` keeps a single machine, which is enough. Fly creates two by default; with Redis they would [share scans](../backend/server/README.md#shared-scans), but each would still keep its own memory cache. If the remote build fails, add `--local-only` to build with your local Docker instead.

Check it:

```bash
curl https://uma-stable-match.fly.dev/api/v1/health
```

It answers `{"status":"ok"}`, and `fly logs` shows JSON lines such as `"message":"using Redis as a shared cache"` and `"message":"scan started"`. Open the site once the scan has finished.

## 6. Ship logs to Axiom

`fly logs` only shows recent lines. Fly's [log shipper](https://github.com/superfly/fly-log-shipper) is a separate app that forwards them to Axiom. Keep its folder outside this repository:

```bash
mkdir -p ~/fly-log-shipper && cd ~/fly-log-shipper
```

```bash
fly launch --no-deploy --image ghcr.io/superfly/fly-log-shipper:latest --name uma-stable-match-logs --region iad
```

In the `fly.toml` this creates, delete any `[http_service]` or `[[services]]` section, as the shipper takes no traffic, and ship only the server's logs:

```toml
[env]
  SUBJECT = "logs.uma-stable-match.>"
```

Set its secrets. `personal` is your organization's slug; `fly orgs list` shows it.

```bash
fly secrets set --stage ORG=personal ACCESS_TOKEN="$(fly tokens create readonly personal)"
```

Then run this, paste `AXIOM_TOKEN=<your token>` and `AXIOM_DATASET=uma-stable-match` on two lines, press Enter, then Ctrl+D:

```bash
fly secrets import --stage
```

```bash
fly deploy --ha=false
```

## 7. Search the logs

Lines appear in the dataset's Stream view within a minute. The shipper sends each server line as a string in `message`, so read its fields with `parse_json`:

```kusto
['uma-stable-match']
| extend log = parse_json(message)
| where log.level == "ERROR"
```

To filter by clicking instead, save `log.level`, `log.message` and `log.circle_id` as virtual fields. The fields are described in the [server README](../backend/server/README.md#logs).

## 8. Get an alert when the site is down

Add an HTTP(s) monitor at an uptime service such as [UptimeRobot](https://uptimerobot.com), checking `https://uma-stable-match.fly.dev/api/v1/health` every 5 minutes, with an email alert.

## 9. Optional: your own domain

```bash
fly certs add <your-domain>
```

Create the DNS records it prints, then point the uptime monitor at the new address.

## Day to day

| Task | Command |
|---|---|
| Deploy a new version | `fly deploy --ha=false` from the repository root |
| Live logs | `fly logs`; Axiom keeps the history |
| Machine and health check status | `fly status` and `fly checks list` |
| Roll back | `fly releases --image`, then `fly deploy --ha=false --image <previous image>` |
| Restart | `fly machine restart` |
| Rotate the API key | Create a new key at uma.moe, then `fly secrets import` with the new value; it redeploys |
| Redis status and logs | `fly status -a uma-stable-match-redis` and `fly logs -a uma-stable-match-redis` |
| Deploy Redis changes | `fly deploy deploy/redis --ha=false`. The server reconnects by itself |
| Restore Redis | `fly volumes snapshots list <volume id>`, then create a volume from a snapshot with `fly volumes create redis_data --snapshot-id <id>` |
