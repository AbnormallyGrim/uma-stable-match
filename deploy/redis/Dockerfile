# Redis for the production server on Fly.io. See docs/deploy-fly.md.
FROM redis:7.4-alpine

# Requires REDIS_PASSWORD, a Fly secret: Redis refuses to start without it. Persistence matches the
# `redis` profile in docker-compose.yml. maxmemory leaves room in a 256 MB machine for rewriting the
# append-only file; volatile-lru only evicts keys that expire. The image's entrypoint hands /data to
# the redis user and drops root.
CMD ["sh", "-c", "sysctl -w vm.overcommit_memory=1 >/dev/null 2>&1 || true; exec docker-entrypoint.sh redis-server --requirepass \"${REDIS_PASSWORD:?REDIS_PASSWORD is not set}\" --dir /data --save '' --appendonly yes --appendfsync everysec --maxmemory 100mb --maxmemory-policy volatile-lru"]
