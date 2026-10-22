# Hosting on Fly.io

How to run a public instance on [Fly.io](https://fly.io), with logs searchable in [Axiom](https://axiom.co), metrics on a Grafana dashboard and an alert when the site goes down. [`fly.toml`](../fly.toml) in the repository root holds the app's settings.

```text
visitors → Fly proxy (HTTPS) → server machine ─ logs → log shipper machine → Axiom
                                     │  ↑
                                     │  uptime monitor checks /api/v1/health
                                     └─ metrics → Fly's Prometheus → Fly's Grafana
```

It runs one always-on machine for the server and one for the log shipper. Fly.io needs a payment card; check its pricing for the current cost of two small machines.

## 1. Install the Fly CLI and sign in

```bash
brew install flyctl
```

On other platforms, follow [Fly's install guide](https://fly.io/docs/flyctl/install/). Then run `fly auth signup`, or `fly auth login` if you have an account.

## 2. Set up Axiom

1. Sign up at [axiom.co](https://axiom.co) and pick a region near where the server runs. `fly.toml` uses `iad` (US East), so pick US East 1. The region can't be changed later.
2. Create a dataset, such as `uma-stable-match`.
3. Under Settings → API tokens, create a token that can only ingest into that dataset. You need it in step 5.

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

## 4. Deploy

```bash
fly deploy --ha=false
```

`--ha=false` keeps a single machine. Fly creates two by default, and without [Redis](../backend/server/README.md#shared-cache) each would scan uma.moe on its own with its own cache. If the remote build fails, add `--local-only` to build with your local Docker instead.

Check it:

```bash
curl https://uma-stable-match.fly.dev/api/v1/health
```

It answers `{"status":"ok"}`, and `fly logs` shows JSON lines such as `"message":"scan started"`. Open the site once the scan has finished.

## 5. Ship logs to Axiom

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

## 6. Search the logs

Lines appear in the dataset's Stream view within a minute. The shipper sends each server line as a string in `message`, so read its fields with `parse_json`:

```kusto
['uma-stable-match']
| extend log = parse_json(message)
| where log.level == "ERROR"
```

To filter by clicking instead, save `log.level`, `log.message` and `log.circle_id` as virtual fields. The fields are described in the [server README](../backend/server/README.md#logs).

## 7. View the metrics

Fly scrapes the server's [metrics](../backend/server/README.md#metrics) every 15 seconds into its managed Prometheus, through the `[metrics]` section of `fly.toml`. They're served on port 9464 of Fly's private network only, never the public address. Fly keeps them for about 15 days, with no extra charge at the time of writing.

To see them on the repository's dashboard:

1. Open [fly-metrics.net](https://fly-metrics.net), Fly's managed Grafana, and sign in with your Fly account.
2. Go to **Dashboards → New → Import**, upload [`observability/grafana-dashboard.json`](../observability/grafana-dashboard.json), choose Fly's Prometheus data source and click **Import**.

Panels fill in within a few minutes of a deploy. To check a single value, query it in **Explore**, such as `scans_total`. Fly doesn't alert on metrics; the uptime monitor in the next step covers the site going down.

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
| Metrics | The dashboard at [fly-metrics.net](https://fly-metrics.net) |
| Machine and health check status | `fly status` and `fly checks list` |
| Roll back | `fly releases --image`, then `fly deploy --ha=false --image <previous image>` |
| Restart | `fly machine restart` |
| Rotate the API key | Create a new key at uma.moe, then `fly secrets import` with the new value; it redeploys |
