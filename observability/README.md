# Local metrics dashboards

A Prometheus config and four Grafana dashboards for watching a server you run yourself: scan progress, uma.moe calls by status code, the server's own requests, caches, Redis and the club pool. Everything runs on your computer and listens on `127.0.0.1` only.

```text
server (/metrics on 127.0.0.1:9464)  ←  Prometheus (127.0.0.1:9090)  ←  Grafana (127.0.0.1:3000)
```

| File | What it is |
|---|---|
| `prometheus.yml` | Scrapes the server's metrics from `127.0.0.1:9464` every 5 seconds |
| `dashboard-overview.json` | Searches, health at a glance, web traffic and logged errors. It asks which Prometheus data source to use and your Axiom organization ID |
| `dashboard-search-quality.json` | How deep into the rankings searches go, and the club pool |
| `dashboard-data-pipeline.json` | Scans, refreshes, caches and calls to uma.moe |
| `dashboard-redis.json` | Redis keys, memory, TTLs, timings and errors |

For traces, see [Traces](#traces). The metrics themselves are described in the [server README](../backend/server/README.md#metrics). On Fly.io, the `[metrics]` section of `fly.toml` sends them to Fly's managed Prometheus instead, and the same dashboards can be imported into Fly's Grafana at [fly-metrics.net](https://fly-metrics.net).

## 1. Install Prometheus and Grafana

| Platform | Prometheus | Grafana |
|---|---|---|
| macOS | `brew install prometheus` | `brew install grafana` |
| Windows | Download the `windows-amd64` zip from [prometheus.io/download](https://prometheus.io/download/) and extract it so `prometheus.exe` is at, for example, `C:\prometheus\prometheus.exe` | Download and run the installer from [grafana.com](https://grafana.com/grafana/download?platform=windows). It runs Grafana as a Windows service |
| Linux | Download the `linux-amd64` archive from [prometheus.io/download](https://prometheus.io/download/) and extract it, for example with `tar xzf prometheus-*.linux-amd64.tar.gz -C ~` | Add Grafana's package repository and install it, following Grafana's guide for [Debian and Ubuntu](https://grafana.com/docs/grafana/latest/setup-grafana/installation/debian/) or [Red Hat and Fedora](https://grafana.com/docs/grafana/latest/setup-grafana/installation/redhat-rhel-fedora/) |

`brew` commands need [Homebrew](https://brew.sh). On Linux, a distribution's own Prometheus package also works, but it starts its own service on port 9090; stop that service, or use another port in step 5.

## 2. Keep Grafana on this computer

Grafana listens on every network interface by default. Restrict it to your computer before starting it.

macOS:

```bash
sed -i '' 's/^;http_addr =.*/http_addr = 127.0.0.1/' "$(brew --prefix)/etc/grafana/grafana.ini"
grep -n '^http_addr' "$(brew --prefix)/etc/grafana/grafana.ini"
```

Linux:

```bash
sudo sed -i 's/^;http_addr =.*/http_addr = 127.0.0.1/' /etc/grafana/grafana.ini
grep -n '^http_addr' /etc/grafana/grafana.ini
```

Both should print `http_addr = 127.0.0.1`.

Windows, in PowerShell run as administrator:

```powershell
Set-Content "C:\Program Files\GrafanaLabs\grafana\conf\custom.ini" "[server]`nhttp_addr = 127.0.0.1"
```

If Grafana is installed somewhere else, put `custom.ini` in the `conf` folder of that installation.

## 3. Start Grafana

| Platform | Start | Stop |
|---|---|---|
| macOS | `brew services start grafana` | `brew services stop grafana` |
| Windows (PowerShell as administrator) | `Restart-Service Grafana` | `Stop-Service Grafana` |
| Linux | `sudo systemctl restart grafana-server` | `sudo systemctl stop grafana-server` |

On Windows and Linux, the restart also picks up the setting from step 2.

Open <http://localhost:3000>, sign in as `admin` with the password `admin`, and choose a new password when asked.

## 4. Turn on the server's metrics

Metrics are off by default. Turn them on in `.env`, then restart the server.

With Docker Compose, set the port to publish on your computer:

```
METRICS_PORT=9464
```

```bash
docker compose up -d
```

Running the server with `cargo run` instead, set the address it listens on:

```
METRICS_ADDR=127.0.0.1:9464
```

Check that metrics are coming out:

```bash
curl -s http://127.0.0.1:9464/metrics | grep scan_phase
```

In PowerShell on Windows:

```powershell
curl.exe -s http://127.0.0.1:9464/metrics | Select-String scan_phase
```

If you pick a port other than 9464, change the target in `prometheus.yml` to match.

## 5. Start Prometheus

From the repository root, in its own terminal.

macOS:

```bash
prometheus --config.file=observability/prometheus.yml --storage.tsdb.path=observability/data --web.listen-address=127.0.0.1:9090
```

Linux, using the folder you extracted Prometheus into:

```bash
~/prometheus-*/prometheus --config.file=observability/prometheus.yml --storage.tsdb.path=observability/data --web.listen-address=127.0.0.1:9090
```

Windows, in PowerShell, using the folder you extracted Prometheus into:

```powershell
C:\prometheus\prometheus.exe --config.file=observability/prometheus.yml --storage.tsdb.path=observability/data --web.listen-address=127.0.0.1:9090
```

Prometheus keeps its data in `observability/data/`, which git ignores. At <http://localhost:9090/targets>, the `uma-stable-match` target should be `UP`.

## 6. Add Prometheus to Grafana

In Grafana, go to **Connections → Data sources → Add data source → Prometheus**. Set the URL to `http://localhost:9090`, then click **Save & test**.

## 7. Import the dashboards

For each `observability/dashboard-*.json` file, go to **Dashboards → New → Import**, upload the file, choose the Prometheus data source from step 6, and click **Import**. The Overview also asks for your **Axiom organization ID** (the part after `app.axiom.co/` in your Axiom URLs; leave it empty without Axiom). Importing a file again later replaces that dashboard rather than adding a copy. The Overview replaces the single dashboard from earlier versions.

Every dashboard links to the other three at the top, keeping the time range. The links find the dashboards by their `uma-stable-match` tag.

## What the dashboards show

| Dashboard | Row | Panels |
|---|---|---|
| Overview | Searches | Searches in the dashboard's time range, the total since launch, the same by status code, and searches per hour |
| | Health | Redis up or down, time since the last successful scan, and uma.moe calls without a 2xx answer in the last hour |
| | Web traffic | Requests per second by route and status code, error responses, response times, rate-limited visitors and a count of each status code in the dashboard's time range |
| | Errors | Warnings and errors logged per hour, and a table of them by the file and line that logged them. Each file links to [Axiom](#links-to-axiom) |
| Search quality | Search depth | How deep into the rankings searches go (worst shortlisted and top-pick rank, median and 90th percentile), the share of searches that reached the slowest clubs in the pool, and searches by deepest shortlisted rank |
| | Club pool | Clubs in the pool, clubs with an exact median, and the calibration factor and samples |
| Data pipeline | Scan and refresh | Phase, refresh counts for joinable clubs (loaded, scheduled and due), clubs listed, time since the last successful scan, scans finished, scan duration, club details loaded as a percentage of the scheduled clubs (in Redis with Redis), what the instance did when a scan was due, how long the longest-waiting due club has waited, and the same counts and percentage loaded for full clubs and for the lower range |
| | Caches | Where trainer paces and club details came from (memory, Redis, uma.moe or an error), and the detail cache and background queue |
| | uma.moe calls | Calls per second by status code and by endpoint and lane, response times, errors per minute by status, and time waiting for the 500 ms spacing |
| Redis | Redis | Keys and keys with a TTL, memory used against the limit, when the stored club details expire (soonest and average), TTLs over time, keys expired and evicted per hour, up or down, operation times and errors |

Per-hour bars line up with whole hours, so they fill in once there's an hour of data. Some panels stay empty until something happens: Redis panels need `REDIS_URL`, error panels need errors, and rates need about a minute of data.

### Links to Axiom

With logs in [Axiom](https://axiom.co), the Overview links straight to them. Each file in the Errors table opens Axiom with a query for that log line over the dashboard's time range, showing every occurrence with its message, error, origin and any trainer, club or request query. **Warnings and errors in Axiom**, at the top of the Overview, opens all of them.

The links use the Axiom organization ID entered on import and the **Axiom dataset** box at the top of the Overview (`uma-stable-match` by default). Axiom opens a query from a link's `initForm` parameter: URL-encoded JSON with the APL query in `apl` and the time range in `queryOptions`.

## Traces

Grafana's `otel-lgtm` image runs Grafana with Tempo in one container, ready to receive traces. It needs [Docker](https://docs.docker.com/get-docker/), and uses port 3001 so it doesn't clash with the Grafana above:

```bash
docker run --rm --name otel-lgtm -p 127.0.0.1:3001:3000 -p 127.0.0.1:4318:4318 grafana/otel-lgtm
```

Add `OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318` to `.env` and restart the server. With Docker Compose, use `http://host.docker.internal:4318` instead.

Open <http://localhost:3001> and go to **Drilldown → Traces**. It ranks span names by duration, and opening a trace shows its spans as a waterfall. Run a few searches first: spans are sent every few seconds. Stop it with Ctrl+C; it keeps no data.

The spans themselves are described in the [server README](../backend/server/README.md#traces).

## Stopping

1. Press Ctrl+C in the Prometheus terminal.
2. Stop Grafana with the command for your platform from [step 3](#3-start-grafana).
3. Remove `METRICS_PORT` or `METRICS_ADDR` from `.env` and restart the server.

To start again with no history, delete `observability/data/`.

## Troubleshooting

| Problem | Fix |
|---|---|
| The target is `DOWN` | The server isn't running, or metrics are off. Check the `curl` command from step 4 |
| `address already in use` | Another program uses that port. Pick another `METRICS_PORT`, or another `--web.listen-address`, and update `prometheus.yml` or the Grafana data source to match |
| A dashboard shows "No data" everywhere | The Grafana data source points somewhere else, or Prometheus isn't running. Use **Save & test** on the data source |

## Changing a dashboard

Edit it in Grafana, then export it with **Export → Export as JSON**, turning on the option to export for sharing externally. That keeps the data source as a choice made on import, rather than your own data source's ID. Check the file holds nothing from your own instance, such as user IDs or your Axiom organization ID, before replacing its `dashboard-*.json` file.
