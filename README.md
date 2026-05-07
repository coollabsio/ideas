# coolLabs · Ideas

A small web app + repository for documenting and upvoting potential upcoming applications.

Live ideas: [github.com/coollabsio/ideas/discussions](https://github.com/coollabsio/ideas/discussions)

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

- Lists ideas from the **Ideas** category of `coollabsio/ideas` GitHub Discussions.
- Supports **GitHub OAuth** login.
- Can temporarily disable OAuth login with `GITHUB_LOGIN_ENABLED=false` while keeping public browsing active.
- Lets signed-in users **upvote** — votes go straight to GitHub via the GraphQL `addUpvote` mutation. No middle layer.
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
                       ────► GraphQL addUpvote ───►   user upvote
                       ◄──── upvoteCount ───────
                                                       
GET /api/discussions ► (anon: 30s cache)                
                       ────► GraphQL discussions ──►   (server PAT)
```

GitHub is the single source of truth for ideas and votes. The app never stores either.

### Local development

1. Create a GitHub OAuth App at https://github.com/settings/developers
   - Homepage URL: `http://localhost:4321`
   - Authorization callback URL: `http://localhost:4321/api/auth/callback`
2. Create a fine-grained PAT scoped to `coollabsio/ideas` with `Discussions: read`
   (or a classic PAT with `public_repo`). This is used for build-time prerender + the anonymous `/api/discussions` cold path.
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
| `GITHUB_CLIENT_ID` | yes | runtime | OAuth App client ID |
| `GITHUB_CLIENT_SECRET` | yes | runtime | OAuth App client secret |
| `GITHUB_LOGIN_ENABLED` | no | build + runtime | Defaults to `true`. Set to `false` to hide sign-in/new-idea UI and make `/api/auth/login` return 503. Anonymous idea listing still works. |
| `GITHUB_TOKEN` | yes | build + runtime | PAT with `Discussions: read` on `coollabsio/ideas`. Used for prerender + anon `/api/discussions`. |
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
      discussions.ts         # live list (anon: 30s cache)
      upvote.ts              # GraphQL addUpvote / removeUpvote
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
    github.ts                # GraphQL queries/mutations
    utils.ts                 # cn() helper
  styles/global.css          # Coolify tokens, .button, .box utilities
public/
  favicon.png, apple-touch-icon.png, og-image.png
data/
  sessions.db                # gitignored; mounted at /app/data in Docker
```

### License

MIT — see [LICENSE](./LICENSE).
