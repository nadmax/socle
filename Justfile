set quiet

# Show available recipes
help:
    just --list

# Run the app locally with cargo
dev:
    cargo run

# Build the release binary
build:
    cargo build --release --locked

# Run all tests
test:
    cargo test

# Format code
fmt:
    cargo fmt

# Run clippy
lint:
    cargo clippy -- -D warnings -W clippy::pedantic

# Start all services
docker-up:
    docker compose up -d

# Stop all services
docker-down:
    docker compose down

# Run all pending migrations
migrate:
    sqlx migrate run

# Revert the last applied migration
migrate-revert:
    sqlx migrate revert

# Create a new reversible migration (usage: just migrate-add <name>)
migrate-add name:
    sqlx migrate add -r {{name}}

# Drop the database, recreate it and run all migrations from scratch
migrate-fresh:
    -sqlx database drop
    sqlx database create
    sqlx migrate run

# Generate the .sqlx query cache for offline builds
prepare:
    cargo sqlx prepare -- --tests

# Verify the .sqlx cache is in sync with current queries
prepare-check:
    cargo sqlx prepare --check -- --tests

# Install git hooks via prek
prek-install:
    prek install
    prek install --hook-type commit-msg

# Run all prek hooks manually
prek-run:
    prek run

# List all configured prek hooks
prek-list:
    prek list

# Validate the prek.toml config file
prek-validate:
    prek validate-config prek.toml

# Auto-update prek hooks to their latest versions
prek-update:
    prek auto-update

# Clean the prek hook cache
prek-cache-clean:
    prek cache clean
