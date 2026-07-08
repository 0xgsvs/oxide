# Runbook — Local Development

Common issues you might hit while developing and how to fix them.

## Postgres connection refused

```text
error: error communicating with database: Connection refused
```

PostgreSQL is not running. Fix:

```bash
# if using compose
docker compose start postgres

# if using local postgres
sudo systemctl start postgresql   # or brew services start postgresql
```

## Postgres connection reset by peer

```text
error: error communicating with database: Connection reset by peer
```

Postgres is still starting up. Wait a few seconds and retry:

```bash
docker compose logs postgres --tail 5   # wait for "database system is ready to accept connections"
```

## Redis connection refused

```text
thread 'main' panicked at 'Failed to connect to Redis: ... Connection refused'
```

Redis is not running. Fix:

```bash
docker compose start redis
```

## Port already in use

```text
thread 'main' panicked at ... AddrInUse: "Address already in use"
```

Something is already using port 3000. Find and stop it:

```bash
lsof -i :3000        # find the PID
kill <PID>           # stop the process
# or use a different port
PORT=3001 mise run dev
```

## Database migration failed

```text
error: error executing migration ...
```

Roll back the last migration, fix the SQL, and reapply:

```bash
sqlx migrate revert
# edit the migration file
sqlx migrate run
```

## Reset the database

Wipe everything and start fresh:

```bash
# drop and recreate via compose
docker compose down -v
docker compose up -d
sleep 5
sqlx migrate run

# or drop via psql
psql postgres://postgres:postgres@localhost:5433/postgres -c "DROP DATABASE oxide_dev"
psql postgres://postgres:postgres@localhost:5433/postgres -c "CREATE DATABASE oxide_dev"
sqlx migrate run
```

## Test database failure

```text
error: database "sqlx_test_..." already exists
```

Leftover from a previous failed test. Clean up:

```bash
psql postgres://postgres:postgres@localhost:5433/postgres -c "
  SELECT 'DROP DATABASE \"' || datname || '\"' FROM pg_database
  WHERE datname LIKE 'sqlx_test_%'
  AND datistemplate = false;
"
# or just let sqlx clean up on next run
```

## Integration tests fail

```text
thread '...' panicked at ... "Email already exists"
```

Tests can collide if run concurrently. Run with a single thread:

```bash
cargo nextest run --test-threads=1
```

## View logs with request IDs

```bash
mise run dev 2>&1 | grep "request_id="
# or filter by a specific ID
mise run dev 2>&1 | grep "request_id=abc-123"
```

## View metrics

```bash
xh get http://localhost:3000/metrics
# specific metrics
xh get http://localhost:3000/metrics | grep http_requests_total
```

## Check service health

```bash
xh get http://localhost:3000/health
# {"status":"ok"}
```

## Run a single test

```bash
cargo nextest run --test integration <test_name>
# example:
cargo nextest run --test integration health_check
```

## Common `sqlx` issues

### Compile-time query check fails

```text
error: unknown column type: ...
```

The PostgreSQL database must be running when you run `cargo check` or `cargo build` because SQLx connects to it at compile time to validate queries.

```bash
docker compose up -d         # start postgres + redis
sqlx migrate run              # ensure schema is up to date
cargo check                   # now it works
```

### Offline mode

If you want to check without a database:

```bash
cargo sqlx prepare --workspace
```

Then unset `DATABASE_URL` and check:

```bash
export SQLX_OFFLINE=true
cargo check
```

## Release build

```bash
mise run release
```

The release profile strips symbols, enables LTO, and uses a single codegen unit for maximum size reduction.

## Full stack restart

```bash
docker compose down -v        # stop everything, wipe volumes
docker compose up -d          # fresh start
sleep 5                       # wait for postgres
sqlx migrate run              # apply schema
mise run dev                  # start the app
```
