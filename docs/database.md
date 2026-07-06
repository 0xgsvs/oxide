# Database Setup

This project uses **PostgreSQL** with **SQLx**.

## Connection URL

The app reads `DATABASE_URL` from the environment. A `.env` file is included for local development.

```
DATABASE_URL=postgres://postgres:postgres@localhost:5432/oxide_dev
```

Format: `postgres://<user>:<password>@<host>:<port>/<database>`

## Option 1: Local PostgreSQL

If you already have PostgreSQL installed:

```bash
# Create the database
createdb oxide_dev

# Run migrations
sqlx migrate run

# Start the app
just run
```

## Option 2: Docker

If you prefer Docker or do not have PostgreSQL installed locally:

```bash
# Start Postgres in Docker
docker run --name oxide-postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=oxide_dev \
  -p 5432:5432 \
  -d postgres:17

# Run migrations
sqlx migrate run

# Start the app
just run
```

To stop and remove the container later:

```bash
docker stop oxide-postgres
docker rm oxide-postgres
```

## Checking the connection

SQLx needs a live database at compile time if you use `query_as!` macros. Before running `cargo build` or `cargo check`, make sure either local Postgres or Docker Postgres is running and migrations are applied.

## Migrations

Migrations live in `migrations/` and are managed by `sqlx-cli`.

```bash
# Create a new migration
sqlx migrate add <name>

# Apply pending migrations
sqlx migrate run

# Revert the last migration
sqlx migrate revert
```
