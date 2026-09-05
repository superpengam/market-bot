# Market Bot operations runbook

Local scaffolding only. This is not a production cloud runbook.

## Start the local stack

From the repository root:

```bash
cp .env.example .env
docker compose -f infra/compose/docker-compose.dev.yml up --build
```

Services:

- API: `http://127.0.0.1:3000` (`GET /healthz` should return `{"status":"ok"}`)
- Web: `http://127.0.0.1:3001`
- PostgreSQL: `127.0.0.1:5432` user `marketbot`, database `marketbot`
- Redis: `127.0.0.1:6379`
- MinIO: `http://127.0.0.1:9000` (console on `9001`)

Payment and logistics environment variables are set to `sandbox`. Do not replace them with production credentials.

OpenSearch is commented out of the compose file because the JVM image is heavy. Uncomment `opensearch` in `infra/compose/docker-compose.dev.yml` when you need a real search cluster.

Stop the stack with `docker compose -f infra/compose/docker-compose.dev.yml down`. Add `-v` only when you intend to wipe local Postgres and MinIO volumes.

## Quality checks

```bash
bash scripts/check.sh
bash scripts/security_check.sh
bash scripts/load_test.sh
npm --prefix apps/web run typecheck
```

`scripts/load_test.sh` prints search, checkout preview, and order-lookup timings when the API is up. If nothing is listening it exits 0 after stating that it would hit those routes.

`scripts/security_check.sh` fails when a runtime `.env` file, private key, live payment secret, or AWS access key is tracked in git.

## Outbox backlog

The worker publishes rows from `outbox_events`. Pending work should drain as `status = 'published'`.

Check the pile-up:

```sql
SELECT status, count(*) AS events, min(created_at) AS oldest, max(attempts) AS max_attempts
FROM outbox_events
GROUP BY status;
```

If `pending` grows:

1. Confirm the worker container is running and its logs are advancing.
2. Confirm `DATABASE_URL` points at the same database the API writes to.
3. Inspect `last_error` on the oldest pending rows and fix the downstream adapter (sandbox payment, object storage, or logistics).
4. Do not delete pending rows. Let the worker retry using `available_at`.

## Dead letters

Rows with `status = 'dead_letter'` exceeded the publish retry budget. They will not be claimed again by the normal loop.

```sql
SELECT event_id, event_type, aggregate_type, aggregate_id, attempts, last_error, created_at
FROM outbox_events
WHERE status = 'dead_letter'
ORDER BY created_at;
```

What to do:

1. Use `event_id` plus `request_id` / `order_id` / `payment_id` in API and worker logs to find the original request.
2. Fix the poison payload or the downstream failure.
3. After a human review, re-queue by setting `status = 'pending'`, `attempts = 0`, `available_at = now()`, and clearing `last_error`.
4. If the business action already happened (payment captured, file delivered), leave the row as `dead_letter` and record the decision. Do not re-publish blindly.

Correlate every investigation with `request_id`, `order_id`, `payment_id`, and `event_id`. Never paste card secrets, payment tokens, or full addresses into tickets or logs.
