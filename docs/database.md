# Database Setup

This project uses **PostgreSQL** with **SQLx**.

## Connection URL

The app reads `DATABASE_URL` from the environment. Which one you set depends on whether you use the Docker compose stack or your local PostgreSQL.

### Docker Compose (recommended)

Starts both PostgreSQL (port 5433) and Redis:

```bash
docker compose up -d
```

```
DATABASE_URL=postgres://postgres:postgres@localhost:5433/oxide_dev
```

### Local PostgreSQL

If you already have PostgreSQL running on the default port 5432:

```
DATABASE_URL=postgres://postgres:postgres@localhost:5432/oxide_dev
```

```bash
# Create the database
createdb oxide_dev

# Run migrations
sqlx migrate run
```

## Checking the connection

SQLx needs a live database at compile time if you use `query_as!` macros. Before running `cargo build` or `cargo check`, make sure either Docker Compose or your local Postgres is running and migrations are applied.

## Migrations

Migrations live in `migrations/` and are managed by `sqlx-cli`.

```bash
# Create a new migration
sqlx migrate add <name>

# Apply pending migrations
sqlx migrate run
```

## Redis

Redis runs on port 6379 via Docker Compose. No manual setup needed.
