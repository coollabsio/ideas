# coolLabs · Ideas

A small self-hostable app for submitting and upvoting potential coolLabs apps.

GitHub is used only for sign-in. Ideas and upvotes are stored in local SQLite.

## Stack

- Rust `axum` API server
- `sqlx` + SQLite migrations
- SvelteKit static SPA embedded into the Rust binary
- GitHub OAuth/App user authorization for identity

## Local development

1. Create a GitHub OAuth app or GitHub App user authorization app.
   - Homepage URL: `http://127.0.0.1:5173`
   - Callback URL: `http://127.0.0.1:5173/api/auth/callback`
2. Copy env and fill values:

```bash
cp .env.example .env
```

3. Run migrations, optionally seed example ideas for local dev, and start the frontend + backend:

```bash
cargo run -p ideas-server -- db migrate
cargo run -p ideas-server -- db seed --db-path ./data/ideas-dev.db
bun run dev
```

The seed command is idempotent and only creates reusable development data. Run it again whenever you want to restore the example users, ideas, and upvotes.

Use the same host in the browser, `PUBLIC_BASE_URL`, and the GitHub callback URL.
For the default Vite dev server, that host is `127.0.0.1`, not `localhost`;
cookies are host-scoped, so mixing them can complete OAuth but leave the UI
looking logged out.

For backend-only iteration without building the frontend each time:

```bash
SKIP_FRONTEND=1 cargo run -p ideas-server -- serve
```

Frontend dev server:

```bash
cd frontend
bun install
bun run dev
```

## Production build

```bash
cargo build --release -p ideas-server
./target/release/ideas serve
```

## Database CLI

```bash
./target/release/ideas db migrate
./target/release/ideas db seed --db-path ./data/ideas-dev.db
./target/release/ideas db revert
./target/release/ideas db info
```

## Versioning and releases

The app has one release version: root `Cargo.toml` `[workspace.package].version`.
All Rust crates inherit it with `version.workspace = true`; the embedded frontend is not versioned separately.

Release tags use `vX.Y.Z` and must match the Cargo workspace version. For example, `version = "0.1.0"` must be released from tag `v0.1.0`.

Release artifacts are built only from tags by `.github/workflows/release.yml`. The workflow:

1. Installs Rust and Bun.
2. Installs frontend dependencies.
3. Verifies the Git tag matches the Cargo workspace version.
4. Runs `cargo fmt --all -- --check`.
5. Runs `cargo clippy --all-targets --all-features -- -D warnings`.
6. Runs `cargo test --all --all-features`.
7. Builds `cargo build --release -p ideas-server`.
8. Publishes `ideas-linux-x86_64.tar.gz` and `ideas-linux-x86_64.tar.gz.sha256` to the GitHub Release.

Manual release checklist:

```bash
# 1. update Cargo.toml [workspace.package].version to X.Y.Z
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-features
cargo build --release -p ideas-server
./target/release/ideas --version

git commit -am "chore: release vX.Y.Z"
git tag -a vX.Y.Z -m "vX.Y.Z"
git push && git push origin vX.Y.Z
```

After the GitHub Release is created, download the artifact and run `./ideas --version`; it must print `ideas X.Y.Z`.

## Environment variables

| Var | Required | Notes |
|---|---:|---|
| `GITHUB_CLIENT_ID` | yes | GitHub OAuth/App client ID. |
| `GITHUB_CLIENT_SECRET` | yes | GitHub OAuth/App client secret. |
| `GITHUB_LOGIN_ENABLED` | no | Defaults to `true`; set `false` to disable login. |
| `IDEAS_MODERATOR_LOGINS` | no | Comma-separated GitHub usernames allowed to mark ideas in progress, close/reopen, and delete any idea. Regular authors can edit/delete only their own ideas. |
| `PUBLIC_BASE_URL` | yes | Public origin; callback is `/api/auth/callback`. |
| `DB_PATH` | no | Defaults to `./data/ideas.db`; Docker sets `/app/data/ideas.db`. |
| `HOST` | no | Defaults to `0.0.0.0`. |
| `PORT` | no | Defaults to `4321`. |
| `SKIP_FRONTEND` | no | Set `1` to skip frontend build in `cargo build`. |

### Coolify / Docker production env

Set these in Coolify:

```env
GITHUB_CLIENT_ID=...
GITHUB_CLIENT_SECRET=...
GITHUB_LOGIN_ENABLED=true
IDEAS_MODERATOR_LOGINS=alice,bob
PUBLIC_BASE_URL=https://ideas.example.com
DB_PATH=/app/data/ideas.db
HOST=0.0.0.0
PORT=4321
```

The GitHub OAuth callback URL must match:

```text
https://ideas.example.com/api/auth/callback
```

Persist `/app/data` as a Coolify volume so SQLite survives redeploys.

Do **not** set `GITHUB_TOKEN`; the app does not need a GitHub PAT.

## Security notes

- Session cookie stores only an opaque `sid`.
- GitHub access tokens are used only during OAuth callback and are not persisted.
- Mutating routes require `x-csrf-token` from `/api/me`.
- Regular authors can edit and delete their own ideas, but only `IDEAS_MODERATOR_LOGINS` users can mark ideas in progress or close/reopen ideas. Moderators can also delete any idea.
- Upvotes are uniquely constrained by `(idea_id, user_id)`.
- Global security headers are applied to API and static responses.
- Login, idea creation, and upvote routes have in-memory rate limits.

## License

Apache-2.0 — see [LICENSE](./LICENSE).
