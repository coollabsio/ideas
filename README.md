# coolLabs · Ideas

A small web app + repository for documenting and upvoting potential upcoming applications.

Live ideas: [github.com/coollabsio/ideas/issues](https://github.com/coollabsio/ideas/issues)

## Requirements for ideas

All apps should be:

- Free
- Open-source
- Self-hostable
- No features behind paywalls (if a hosted version exists)
- Cool (this is important!)

## Why?

Since [Coolify](https://coolify.io?ref=coollabsideas) became ramen profitable, I've realized I should create more cool apps to help people.

---

## The web app

A small Astro + React app that:

- Lists ideas from `coollabsio/ideas` GitHub Issues with the `idea` label.
- Supports **GitHub OAuth** login.
- Can temporarily disable OAuth login with `GITHUB_LOGIN_ENABLED=false` while keeping public browsing active.
- Lets signed-in users **upvote** — votes are GitHub Issue `+1` reactions via the REST API.
- Preserves migrated GitHub Discussion votes as legacy vote metadata in each issue body.
- Renders the list at **build time** for instant first paint, then refreshes counts on the client.
- Uses **SQLite** only for OAuth session storage (no copy of ideas or votes).

### Stack

- [Astro 5](https://astro.build) (`output: 'server'`) + [`@astrojs/node`](https://docs.astro.build/en/guides/integrations-guide/node/)
- [React 19](https://react.dev) for the auth island + shadcn-style primitives (`cva`, `clsx`, `tailwind-merge`, `lucide-react`)
- [Tailwind CSS 3](https://tailwindcss.com) with the [Coolify](https://coolify.io) design tokens (Geist Sans + Geist Mono, dark-first, 4px `rounded-sm` radii, purple/yellow accent swap)
- [`bun:sqlite`](https://bun.com/docs/api/sqlite) (Bun's built-in SQLite) for OAuth session storage
- [Bun](https://bun.com) ≥ 1.3 for install + dev + production runtime (scripts use `bun --bun astro …` to force Bun runtime over the `astro` shebang)

### Architecture

```
Browser                Astro/Bun (server)               GitHub
───────                ─────────────────                ──────
GET /          ───►    prerendered HTML
                                                       
GET /api/me    ───►    sessions table (SQLite)         
                       ↓ {user, csrfToken}
                                                       
POST /api/upvote ─►    sessions table (lookup token)
                       ↓ x-csrf-token check
                       ────► REST issue reaction ─►   user +1
                       ◄──── upvoteCount ───────
                                                       
GET /api/discussions ► (anon: 30s cache)                
                       ────► REST issues ────────►   (server PAT)
```

GitHub Issues are the single source of truth for idea content and new votes. Migrated Discussion upvotes are preserved as `Legacy Discussion upvotes: N` metadata and included in app totals.

### Local development

1. Create a GitHub App at https://github.com/settings/apps/new
   - Homepage URL: `http://localhost:4321`
   - Callback URL: `http://localhost:4321/api/auth/callback`
   - Repository permissions: `Metadata: read`, `Issues: read and write`
   - Install it only on `coollabsio/ideas`
   - Use the GitHub App **Client ID** and **Client secret** for `GITHUB_CLIENT_ID` / `GITHUB_CLIENT_SECRET`; this app does not send OAuth scopes.
2. Create a fine-grained PAT scoped to `coollabsio/ideas` with `Issues: read`
   This server token is separate from user OAuth and is used only for build-time prerender + the anonymous `/api/discussions` cold path.
3. Copy `.env.example` to `.env` and fill in `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_TOKEN`.
   - Set `GITHUB_LOGIN_ENABLED=false` if you need to temporarily disable sign-in and new authenticated actions.
4. Install and run (requires [Bun](https://bun.com) ≥ 1.3):

```bash
bun install
bun run dev
# http://localhost:4321
```

### Production build

```bash
bun run build
bun ./dist/server/entry.mjs
```

### Deploy (Coolify / Docker)

```bash
docker build -t coollabs-ideas --build-arg GITHUB_TOKEN=$GITHUB_TOKEN .
docker run --rm -p 4321:4321 \
  -e GITHUB_CLIENT_ID=... \
  -e GITHUB_CLIENT_SECRET=... \
  -e GITHUB_TOKEN=... \
  -e PUBLIC_BASE_URL=https://ideas.example.com \
  -v $(pwd)/data:/app/data \
  coollabs-ideas
```

Mount `/app/data` to a persistent volume so user sessions survive redeploys.

The container exposes a `HEALTHCHECK` against `GET /api/health` (Coolify-compatible). Configure Coolify health check path: `/api/health`, port `4321`, expected status `200`.

### Environment variables

| Var | Required | Used at | Notes |
|---|---|---|---|
| `GITHUB_CLIENT_ID` | yes | runtime | GitHub App client ID |
| `GITHUB_CLIENT_SECRET` | yes | runtime | GitHub App client secret |
| `GITHUB_LOGIN_ENABLED` | no | build + runtime | Defaults to `true`. Set to `false` to hide sign-in/new-idea UI and make `/api/auth/login` return 503. Anonymous idea listing still works. |
| `GITHUB_TOKEN` | yes | build + runtime | PAT with `Issues: read` on `coollabsio/ideas`. Used for prerender + anon `/api/discussions`; migration additionally needs issue write. |
| `PUBLIC_BASE_URL` | yes | runtime | Public origin; must match the OAuth callback URL |
| `DB_PATH` | no | runtime | Defaults to `./data/sessions.db` |
| `PORT` / `HOST` | no | runtime | Defaults to `4321` / `0.0.0.0` |

### Security notes

- Session cookie holds an **opaque random `sid`** only; access tokens are stored server-side in SQLite. Cookie flags: `HttpOnly`, `Secure` (when `PUBLIC_BASE_URL` is HTTPS), `SameSite=Lax`, `Max-Age=7d`.
- Per-session **CSRF token** is required as `x-csrf-token` header on `POST /api/upvote`.
- OAuth flow uses one-time `state` nonces stored in SQLite, consumed and time-bounded (10 min).
- Session sweep runs at startup and every hour to delete expired rows.

### Project layout

```
src/
  pages/
    index.astro              # build-time prerendered list + auth island
    api/
      auth/{login,callback,logout}.ts
      me.ts                  # auth probe + csrf token
      discussions.ts         # live issue list (anon: 30s cache)
      upvote.ts              # REST issue +1 reaction create/delete
      health.ts              # liveness probe (200 OK + DB ping)
  components/
    IdeaCard.astro
    AuthSlot.tsx             # React island, client:load
    ideas-island.ts          # vanilla TS upvote handler
    ui/{button,avatar}.tsx   # shadcn-style primitives
  lib/
    config.ts                # env validation (lazy)
    db.ts                    # bun:sqlite + schema + sweeper
    session.ts               # session + oauth_state CRUD
    github.ts                # GitHub REST issue/reaction helpers + migration GraphQL read
    utils.ts                 # cn() helper
  styles/global.css          # Coolify tokens, .button, .box utilities
public/
  favicon.png, apple-touch-icon.png, og-image.png
data/
  sessions.db                # gitignored; mounted at /app/data in Docker
  discussion-issue-map.json  # generated by migration script
```

### Migrating existing Discussions to Issues

Run a dry-run first:

```bash
bun run migrate:issues
```

Then create issues and write `data/discussion-issue-map.json`:

```bash
bun run migrate:issues --apply
```

Each migrated issue gets `idea` and `migrated-from-discussion` labels, a backlink to the original Discussion, and a `Legacy Discussion upvotes: N` marker. The app displays `legacy Discussion upvotes + GitHub Issue +1 reactions`; GitHub itself only shows new Issue reactions.

### License

MIT — see [LICENSE](./LICENSE).
