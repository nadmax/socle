# AGENTS.md

Compact guidance for AI coding agents working in this repo. Only repo-specific
facts that an agent would likely miss — not generic language advice.

## Project

**YAIMA** — Yet Another Identity Management API. Axum + Tokio, PostgreSQL 18+
via SQLx (compile-time-checked, offline mode), Redis, JWT-based auth with
refresh-token rotation, OAuth 2.0 (Google, GitHub), role-based access
(Guest/User/Admin), OpenAPI spec + Swagger UI at `/apidocs`.

## Setup essentials

```sh
cargo install sqlx-cli --no-default-features --features postgres
cargo install prek
make prek-install          # once, before any changes
cp .env.example .env       # then fill in: DATABASE_URL, JWT_SECRET (≥32 chars), VALKEY_URL
make docker-up             # starts Postgres + Valkey containers
make migrate               # apply migrations (server also auto-migrates on start)
make prepare               # regenerate .sqlx offline cache (includes --tests flag)
```

**Valkey is always required** — the app creates a connection pool at startup even
without OAuth providers configured.

## Key commands

| Command | What it does |
|---|---|
| `make dev` | `cargo run` |
| `make build` | Release build (`--release --locked`) |
| `make test` | Full suite (unit + integration) |
| `make lint` | `cargo clippy` with `-D warnings -W clippy::pedantic` |
| `make fmt` / `make prepare` | Format code / Regenerate `.sqlx` cache |
| `make prepare-check` | Verify `.sqlx` cache is fresh (also enforced by CI) |
| `make migrate-add` / `make migrate-revert` / `make migrate-fresh` | Migration helpers |

**If you add or change a SQL query:** run `make prepare` and commit the `.sqlx`
changes. CI uses `SQLX_OFFLINE: true` and requires an up-to-date cache.

## Project structure

```
src/
├── main.rs           # entrypoint — wires Router, starts server (auto-migrates DB)
├── lib.rs            # re-exports public surface; used by integration tests
├── config.rs         # typed env-based config via `envy`
├── state.rs          # AppState shared via Axum `with_state`
├── middleware.rs     # auth extractors: AuthUser, RequireUser, RequireAdmin
├── models.rs         # domain types + SQLx mappings
├── errors.rs         # AppError enum (thiserror, each variant → HTTP status + stable code)
├── routes/{mod, auth, users, admin}.rs
└── services/{mod, auth, token, user, admin, oauth}.rs

tests/
├── common/mod.rs     # test helpers: test_app(), test_server(), register_user(), make_admin()
├── errors.rs, middleware.rs, models.rs
├── routes/           # integration tests mirror src/routes/
└── services/         # unit tests mirror src/services/
```

The `tests/` tree **deliberately mirrors** `src/`. Put tests in the
corresponding location.

## Error handling

All errors are in `src/errors.rs`:

- `AppError` (thiserror) — each variant maps to an HTTP status and a stable
  string code (e.g. `INVALID_CREDENTIALS`, `REFRESH_TOKEN_INVALID`). Handlers
  return `Result<_, AppError>` directly.
- `OAuthError` — separate enum wrapped by `AppError::OAuth`; delegates its own
  status/code mapping.
- **Don't change existing error codes** — clients depend on them.
- **Don't add standalone error types** — extend these enums.

## Testing

```sh
make docker-up   # Postgres + Redis must be running
make migrate     # apply migrations (tests also auto-migrate their pool)
make test
```

- Integration tests spin up a real app against a real DB.
- Test pool picks up `TEST_DATABASE_URL` with fallback to `DATABASE_URL`.
- Test naming: `<subject>_should_<expected_outcome>_when_<condition>`.
- Helper `make_admin()` promotes via raw SQL then logs in for a fresh token.

## CI quirks

CI runs `cargo fmt --check`, `cargo build`, `cargo test` (no `make` wrapper).
It sets `SQLX_OFFLINE: true` and a dummy `JWT_SECRET: test` — no Docker, no
database needed for compilation.

## OAuth

Routes at `/auth/{provider}` (authorize), `/auth/{provider}/callback`,
`/auth/connections`, `/auth/connections/{provider}` (unlink). Providers:
Google, GitHub. Configured via `OAUTH_GOOGLE_*` / `OAUTH_GITHUB_*` env vars.
State is stored in Redis (PKCE + CSRF), consumed atomically, 10-minute TTL.

## Hard boundaries

- Don't hand-edit `.sqlx/` files — run `make prepare`.
- Don't add standalone error types — extend `AppError` / `OAuthError` in `errors.rs`.
- Don't weaken role checks (`AuthUser`/`RequireUser`/`RequireAdmin` in middleware).
- Don't change existing client-facing error codes.
- Don't run `make migrate-fresh` on anything but a local dev DB.
- Don't disable `clippy::pedantic` repo-wide — use `#[expect(...)]` on the site.
- Don't bypass prek hooks (`--no-verify`).

## Git conventions

Branch: `<type>/<description>` where type is `feat|fix|chore|docs|refactor|test`.
Commits: [Conventional Commits](https://www.conventionalcommits.org/).

Before PR:

```sh
make fmt && make lint && make prepare-check && make test && make prek-run
```
