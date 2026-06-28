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
just prek-install          # once, before any changes
cp .env.example .env       # then fill in: DATABASE_URL, JWT_SECRET (≥32 chars), VALKEY_URL
just docker-up             # starts Postgres + Valkey containers
just migrate               # apply migrations (server also auto-migrates on start)
just prepare               # regenerate .sqlx offline cache (includes --tests flag)
```

**Valkey is always required** — the app creates a connection pool at startup even
without OAuth providers configured.

## Key commands

| Command | What it does |
|---|---|
| `just dev` | `cargo run` |
| `just build` | Release build (`--release --locked`) |
| `just test` | Full suite (unit + integration) |
| `just lint` | `cargo clippy` with `-D warnings -W clippy::pedantic` |
| `just fmt` / `just prepare` | Format code / Regenerate `.sqlx` cache |
| `just prepare-check` | Verify `.sqlx` cache is fresh (also enforced by CI) |
| `just migrate-add` / `just migrate-revert` / `just migrate-fresh` | Migration helpers |

**If you add or change a SQL query:** run `just prepare` and commit the `.sqlx`
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
├── mod.rs            # integration test crate root (single binary via `autotests = false`)
├── common/mod.rs     # test helpers: test_app(), test_server(), register_user(), make_admin()
├── config.rs, errors.rs, middleware.rs, models.rs, state.rs, shutdown.rs
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
just docker-up   # Postgres + Redis must be running
just migrate     # apply migrations (tests also auto-migrate their pool)
just test
```

- Integration tests spin up a real app against a real DB.
- Test pool picks up `TEST_DATABASE_URL` with fallback to `DATABASE_URL`.
- Test naming: `<subject>_should_<expected_outcome>_when_<condition>`.
- Helper `make_admin()` promotes via raw SQL then logs in for a fresh token.

## CI quirks

CI runs `cargo fmt --check`, `cargo build`, `cargo test` (no `just` wrapper).
It sets `SQLX_OFFLINE: true` and a dummy `JWT_SECRET: test` — no Docker, no
database needed for compilation.

## OAuth

Routes at `/auth/{provider}` (authorize), `/auth/{provider}/callback`,
`/auth/connections`, `/auth/connections/{provider}` (unlink). Providers:
Google, GitHub. Configured via `OAUTH_GOOGLE_*` / `OAUTH_GITHUB_*` env vars.
State is stored in Redis (PKCE + CSRF), consumed atomically, 10-minute TTL.

## Hard boundaries

- Don't hand-edit `.sqlx/` files — run `just prepare`.
- Don't add standalone error types — extend `AppError` / `OAuthError` in `errors.rs`.
- Don't weaken role checks (`AuthUser`/`RequireUser`/`RequireAdmin` in middleware).
- Don't change existing client-facing error codes.
- Don't run `just migrate-fresh` on anything but a local dev DB.
- Don't disable `clippy::pedantic` repo-wide — use `#[expect(...)]` on the site.
- Don't bypass prek hooks (`--no-verify`).

## Git conventions

- **Branch:** `<type>/<description>` where type is `feat|fix|chore|docs|refactor|test`.
- **Commits:** [Conventional Commits](https://www.conventionalcommits.org/) — always include a scope (e.g. `feat(auth):`, `fix(oauth):`).
- **PR description:** Markdown, following `.github/PULL_REQUEST_TEMPLATE.md` — fill in
  What, Why, Changes, Testing, and optionally Notes.

Before PR:

```sh
just fmt && just lint && just prepare-check && just test && just prek-run
```
