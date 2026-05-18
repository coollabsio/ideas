# coolLabs Ideas architecture

`ideas` is a self-hostable Rust + SvelteKit single-binary app.

## Source of truth

- GitHub is used only for OAuth identity.
- `IDEAS_MODERATOR_LOGINS` is a comma-separated list of GitHub usernames with moderation permission.
- SQLite stores users, sessions, ideas, and upvotes.
- No GitHub Issues, Discussions, reactions, PAT, or repository sync is required at runtime.

## Backend

- `crates/server`: axum binary, clap CLI, GitHub OAuth, embedded frontend.
- `crates/storage`: sqlx SQLite store and embedded migrations.
- `crates/domain`: serializable API/domain types.

Routes:

- `GET /api/health`
- `GET /api/me`
- `GET /api/auth/login`
- `GET /api/auth/callback`
- `POST /api/auth/logout`
- `GET /api/ideas`
- `POST /api/ideas`
- `PATCH /api/ideas/:id`
- `DELETE /api/ideas/:id` — authors for own ideas; moderators for any idea
- `PATCH /api/ideas/:id/status` — moderators only; accepts `open`, `inprogress`, or `closed`
- `POST /api/ideas/:id/upvote`

Authorization:

- Idea authors can edit and delete their own ideas.
- Idea authors cannot mark ideas in progress or close/reopen ideas unless their GitHub login is in `IDEAS_MODERATOR_LOGINS`.
- Moderators can mark ideas in progress, close/reopen, and delete any idea.

## Database

Migrations live in `crates/storage/migrations` and are embedded into the storage crate for runtime migration/revert operations.

Tables:

- `users`: GitHub identity mirror.
- `sessions`: opaque sid cookie maps to a local user and CSRF token; GitHub OAuth tokens are not persisted.
- `oauth_state`: short-lived login CSRF state.
- `ideas`: local idea content and status.
- `upvotes`: unique `(idea_id, user_id)` vote records.

CLI:

```bash
ideas db migrate
ideas db revert
ideas db info
```

## Frontend

`frontend/` is a SvelteKit SPA built with adapter-static fallback `200.html` and embedded into the Rust binary by `rust-embed`.

## Versioning and release workflow

- The single source of truth for the app version is root `Cargo.toml` `[workspace.package].version`.
- `ideas-domain`, `ideas-storage`, and `ideas-server` all use `version.workspace = true`.
- The shipped binary is `ideas`; `ideas --version` is provided by clap and must match the workspace version.
- The frontend package is an embedded implementation detail and is not versioned independently.
- GitHub release tags use `vX.Y.Z` and must match the Cargo version exactly.
- `.github/workflows/release.yml` builds artifacts only from tags, packages `target/release/ideas` as `ideas-linux-x86_64.tar.gz`, writes a SHA-256 checksum, and uploads both to the GitHub Release.

Local release gates mirror CI:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-features
cargo build --release -p ideas-server
./target/release/ideas --version
```
