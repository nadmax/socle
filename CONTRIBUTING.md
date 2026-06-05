# Contributing

Thank you for taking the time to contribute! This document covers everything you need to know to go from a fresh clone to an accepted pull request.

---

## Table of Contents

- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Project Structure](#project-structure)
- [Coding Conventions](#coding-conventions)
- [Running Tests](#running-tests)
- [Submitting Changes](#submitting-changes)

---

## Getting Started

Local setup is covered in the [README Quick Start](https://github.com/nadmax/yaima/blob/master/README.md#quick-start). Follow those instructions to get the project building and the database running before working on anything else.

---

## Development Workflow

All common tasks are wrapped in `make` targets. Run `make help` at any time to see the full list with descriptions.

### Application

| Target | What it does |
|---|---|
| `make dev` | Run the app locally with `cargo run` |
| `make build` | Compile a release binary (`--release --locked`) |
| `make test` | Run the full test suite (unit + integration) |
| `make lint` | Run `cargo clippy` — all warnings and pedantic lints are errors |

### Database & Migrations

| Target | What it does |
|---|---|
| `make db` | Start only the database container |
| `make db-reset` | Wipe all volumes and restart the database |
| `make down` | Stop all Compose services |
| `make migrate` | Apply all pending migrations |
| `make migrate-revert` | Revert the last applied migration |
| `make migrate-add` | Prompt for a name and create a new reversible migration file |
| `make migrate-fresh` | Drop the database, recreate it, and replay all migrations from scratch |

### SQLx Offline Cache

The project uses `sqlx` with compile-time query checking. The `.sqlx` query cache must be kept in sync whenever you add or change a SQL query.

| Target | What it does |
|---|---|
| `make prepare` | Regenerate the `.sqlx` cache from current queries |
| `make prepare-check` | Verify the cache matches current queries (runs in CI) |

Always run `make prepare` after touching a SQL query and commit the resulting `.sqlx` changes alongside your code. CI runs `make prepare-check` and will fail if the cache is stale.

### Git Hooks (prek)

The repository uses [`prek`](https://github.com/j178/prek) to manage Git hooks declared in `prek.toml`. The hooks run formatting and linting checks automatically before each commit, so CI should never catch something your local environment didn't.

| Target | What it does |
|---|---|
| `make prek-install` | Install the Git hooks — **run this once after cloning** |
| `make prek-run` | Run all hooks manually against the working tree |
| `make prek-list` | List every configured hook and its status |
| `make prek-validate` | Validate `prek.toml` for syntax errors |
| `make prek-update` | Auto-update hooks to their latest versions |
| `make prek-cache-clean` | Clear the prek hook cache |

After cloning, run `make prek-install` before making any changes so the pre-commit hooks are active.

---

## Project Structure

```
src/
├── main.rs          # Binary entry point; wires up the router and starts the server
├── lib.rs           # Crate root; re-exports the public surface used by integration tests
├── config.rs        # Typed configuration loaded from environment variables
├── state.rs         # Shared application state (database pool, config, etc.) passed via Axum extensions
├── middleware.rs     # Tower middleware layers (auth extraction, request tracing, …)
├── models.rs        # Domain types and their database mappings
├── errors.rs        # Crate-wide error type; see Coding Conventions below
├── routes/
│   ├── mod.rs       # Router assembly — combines all sub-routers into one
│   ├── auth.rs      # Authentication endpoints (login, refresh, logout)
│   ├── users.rs     # User-facing endpoints
│   └── admin.rs     # Admin-only endpoints
└── services/
    ├── mod.rs        # Re-exports all services
    ├── auth.rs       # Authentication business logic
    ├── token.rs      # JWT creation and validation
    ├── user.rs       # User management business logic
    └── admin.rs      # Admin business logic

tests/
├── common/mod.rs    # Shared test helpers (app fixture, database seeding, HTTP client)
├── errors.rs        # Tests for error type behaviour and HTTP mapping
├── middleware.rs     # Tests for middleware layers in isolation
├── models.rs        # Tests for model conversions and validation
├── routes/          # Integration tests — mirror of src/routes/
└── services/        # Unit tests for service layer — mirror of src/services/
```

The `tests/` tree deliberately mirrors `src/`. When you add a new module or change existing behaviour, put the corresponding test file in the matching location under `tests/`.

---

## Coding Conventions

### Idiomatic Rust

- Prefer borrowing (`&T`, `&str`, `&[T]`) over cloning. Only clone when ownership transfer is genuinely needed.
- Use the `?` operator for error propagation instead of explicit `match` chains.
- Favour iterators over manual `for` loops and avoid intermediate `.collect()` calls where possible.
- Do not use `unwrap()` or `expect()` outside of tests. Propagate errors up to the handler layer.
- Avoid `panic!` in any code path that could be reached in production.

### Error Handling

All error types are defined in `src/errors.rs`. The pattern used throughout this project is:

- **`thiserror`** for structured, typed errors with `#[derive(thiserror::Error)]`.
- Each variant maps to an HTTP status code so that route handlers can return `Result<_, AppError>` directly without any extra conversion logic.
- When adding a new error condition, add a variant to the appropriate error enum in `errors.rs` rather than introducing a new ad-hoc type.
- Never swallow errors silently. If a branch genuinely cannot fail, document why with a `// SAFETY:` or `// Invariant:` comment.

### Linting

The project runs Clippy with `-D warnings -W clippy::pedantic`, which means both standard and pedantic lints are treated as hard errors. All Clippy warnings must be resolved — do not suppress them with `#[allow(...)]`. If a lint is a genuine false positive, use `#[expect(clippy::lint_name)]` with a comment explaining why.

Run `make lint` locally before pushing. The prek pre-commit hook enforces the same check.

### Formatting

Code formatting is enforced by `rustfmt` via the prek pre-commit hook. After running `make prek-install`, every commit is checked automatically. To format manually, run:

```bash
make fmt
```

### Style Rules

- **Naming** — follow standard Rust conventions: `snake_case` for functions and variables, `UpperCamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- **TODOs** — every `TODO` comment must reference an open issue: `// TODO(#123): short description`.

### Documentation

- Public items (types, functions, trait impls) must have `///` doc comments explaining what they do and any invariants the caller must uphold.
- Inline `//` comments explain *why*, not *what*. Avoid restating code in prose.

---

## Running Tests

```sh
# Run the full suite (unit + integration)
make test

# Run only unit tests (no database required)
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run a specific test by name
cargo test <test_name>

# Run tests with output printed (useful for debugging)
cargo test -- --nocapture
```

Integration tests under `tests/` spin up a real application instance against a test database. Before running them, make sure the database container is up and migrations are applied:

```sh
make db       # start the database container
make migrate  # apply any pending migrations
make test     # now run the full suite
```

If your schema is out of date or you want a clean slate, use `make migrate-fresh` to drop and recreate the database before running tests.

Test names follow the pattern `<subject>_should_<expected_outcome>_when_<condition>` — for example, `login_should_return_401_when_password_is_wrong`. Aim for one logical assertion per test.

---

## Submitting Changes

### Branching Strategy

Branch off `master` for all changes:

```
git checkout -b <type>/<short-description>
```

Common branch prefixes:

| Prefix | Use for |
|---|---|
| `feat/` | New features |
| `fix/` | Bug fixes |
| `chore/` | Tooling, dependencies, CI |
| `docs/` | Documentation only |
| `refactor/` | Code restructuring without behaviour change |
| `test/` | Adding or improving tests |

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<optional scope>): <short summary in imperative mood>

<optional body — wrap at 72 chars>

<optional footer: BREAKING CHANGE or Closes #issue>
```

Examples:

```
feat(auth): add refresh token rotation

fix(services/token): return 401 instead of 500 on expired JWT

Closes #42
```

Keep the subject line under 72 characters and use the imperative mood ("add", "fix", "remove" — not "added", "fixes", "removed").

### Pull Request Checklist

Before opening a PR, confirm that all of the following are true:

- [ ] `make prek-run` passes with no failures (formatting + linting)
- [ ] `make lint` passes with no warnings
- [ ] `make test` passes locally against a freshly migrated database
- [ ] `make prepare-check` passes (`.sqlx` cache is up to date)
- [ ] New behaviour is covered by tests in the appropriate `tests/` sub-directory
- [ ] Public API changes include updated `///` doc comments
- [ ] The PR description explains *what* changed and *why*
- [ ] Related issue(s) are referenced in the PR description (`Closes #N`)

PRs that fail CI or are missing tests will not be merged until those issues are resolved.
