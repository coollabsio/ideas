use chrono::{DateTime, Utc};
use ideas_domain::{Comment, Idea, IdeaStatus, PublicUser, Session, User};
use sqlx::{
    migrate::{Migrate, MigrateError, Migration, MigrationType, Migrator},
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

const INIT_UP_SQL: &str = include_str!("../migrations/20260507120000_init.up.sql");
const INIT_DOWN_SQL: &str = include_str!("../migrations/20260507120000_init.down.sql");
const ADD_INPROGRESS_STATUS_UP_SQL: &str =
    include_str!("../migrations/20260518120000_add_inprogress_status.up.sql");
const ADD_INPROGRESS_STATUS_DOWN_SQL: &str =
    include_str!("../migrations/20260518120000_add_inprogress_status.down.sql");
const ADD_COMMENTS_UP_SQL: &str = include_str!("../migrations/20260518130000_add_comments.up.sql");
const ADD_COMMENTS_DOWN_SQL: &str =
    include_str!("../migrations/20260518130000_add_comments.down.sql");
const ADD_COMMENT_UPVOTES_UP_SQL: &str =
    include_str!("../migrations/20260518140000_add_comment_upvotes.up.sql");
const ADD_COMMENT_UPVOTES_DOWN_SQL: &str =
    include_str!("../migrations/20260518140000_add_comment_upvotes.down.sql");
const ADD_IDEA_PROBLEM_UP_SQL: &str =
    include_str!("../migrations/20260519120000_add_idea_problem.up.sql");
const ADD_IDEA_PROBLEM_DOWN_SQL: &str =
    include_str!("../migrations/20260519120000_add_idea_problem.down.sql");

fn embedded_migrator() -> Migrator {
    Migrator {
        migrations: Cow::Owned(vec![
            Migration::new(
                20260507120000,
                Cow::Borrowed("init"),
                MigrationType::ReversibleUp,
                Cow::Borrowed(INIT_UP_SQL),
                INIT_UP_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260507120000,
                Cow::Borrowed("init"),
                MigrationType::ReversibleDown,
                Cow::Borrowed(INIT_DOWN_SQL),
                INIT_DOWN_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260518120000,
                Cow::Borrowed("add_inprogress_status"),
                MigrationType::ReversibleUp,
                Cow::Borrowed(ADD_INPROGRESS_STATUS_UP_SQL),
                ADD_INPROGRESS_STATUS_UP_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260518120000,
                Cow::Borrowed("add_inprogress_status"),
                MigrationType::ReversibleDown,
                Cow::Borrowed(ADD_INPROGRESS_STATUS_DOWN_SQL),
                ADD_INPROGRESS_STATUS_DOWN_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260518130000,
                Cow::Borrowed("add_comments"),
                MigrationType::ReversibleUp,
                Cow::Borrowed(ADD_COMMENTS_UP_SQL),
                ADD_COMMENTS_UP_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260518130000,
                Cow::Borrowed("add_comments"),
                MigrationType::ReversibleDown,
                Cow::Borrowed(ADD_COMMENTS_DOWN_SQL),
                ADD_COMMENTS_DOWN_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260518140000,
                Cow::Borrowed("add_comment_upvotes"),
                MigrationType::ReversibleUp,
                Cow::Borrowed(ADD_COMMENT_UPVOTES_UP_SQL),
                ADD_COMMENT_UPVOTES_UP_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260518140000,
                Cow::Borrowed("add_comment_upvotes"),
                MigrationType::ReversibleDown,
                Cow::Borrowed(ADD_COMMENT_UPVOTES_DOWN_SQL),
                ADD_COMMENT_UPVOTES_DOWN_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260519120000,
                Cow::Borrowed("add_idea_problem"),
                MigrationType::ReversibleUp,
                Cow::Borrowed(ADD_IDEA_PROBLEM_UP_SQL),
                ADD_IDEA_PROBLEM_UP_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260519120000,
                Cow::Borrowed("add_idea_problem"),
                MigrationType::ReversibleDown,
                Cow::Borrowed(ADD_IDEA_PROBLEM_DOWN_SQL),
                ADD_IDEA_PROBLEM_DOWN_SQL.starts_with("-- no-transaction"),
            ),
        ]),
        ..Migrator::DEFAULT
    }
}

fn checksum_hex(checksum: &[u8]) -> String {
    checksum.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn run_lenient_migrations(pool: &SqlitePool) -> std::result::Result<(), MigrateError> {
    let migrator = embedded_migrator();
    let mut conn = pool.acquire().await?;
    let conn = &mut *conn;

    conn.lock().await?;
    let result = async {
        conn.ensure_migrations_table().await?;

        if let Some(version) = conn.dirty_version().await? {
            return Err(MigrateError::Dirty(version));
        }

        let applied_migrations = conn.list_applied_migrations().await?;
        let up_migrations: HashMap<_, _> = migrator
            .iter()
            .filter(|migration| !migration.migration_type.is_down_migration())
            .map(|migration| (migration.version, migration))
            .collect();
        let applied_versions: HashSet<_> = applied_migrations
            .iter()
            .map(|migration| migration.version)
            .collect();

        for applied_migration in &applied_migrations {
            if !up_migrations.contains_key(&applied_migration.version) {
                tracing::warn!(
                    version = applied_migration.version,
                    "applied migration is missing from embedded migrations; skipping"
                );
            }
        }

        for migration in migrator.iter() {
            if migration.migration_type.is_down_migration() {
                continue;
            }

            if let Some(applied_migration) = applied_migrations
                .iter()
                .find(|applied| applied.version == migration.version)
            {
                if migration.checksum != applied_migration.checksum {
                    tracing::warn!(
                        version = migration.version,
                        description = %migration.description,
                        stored_checksum = %checksum_hex(&applied_migration.checksum),
                        embedded_checksum = %checksum_hex(&migration.checksum),
                        "applied migration checksum differs from embedded migration; skipping already-applied migration"
                    );
                }
                continue;
            }

            if applied_versions.iter().any(|version| *version > migration.version) {
                tracing::warn!(
                    version = migration.version,
                    description = %migration.description,
                    "migration is older than an already-applied migration but is missing; applying pending migration"
                );
            }

            conn.apply(migration).await?;
        }

        Ok(())
    }
    .await;
    let unlock_result = conn.unlock().await;

    match (result, unlock_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), _) => Err(err),
        (Ok(()), Err(err)) => Err(err),
    }
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevSeedReport {
    pub users: usize,
    pub ideas: usize,
    pub upvotes: usize,
}

struct SeedUser {
    github_id: i64,
    login: &'static str,
    avatar_url: &'static str,
}

struct SeedIdea {
    title: &'static str,
    body: &'static str,
    problem: &'static str,
    author_login: &'static str,
    upvoter_logins: &'static [&'static str],
    closed: bool,
}

const DEV_SEED_USERS: &[SeedUser] = &[
    SeedUser {
        github_id: 90_000_001,
        login: "coollabs",
        avatar_url: "https://avatars.githubusercontent.com/u/90269785?v=4",
    },
    SeedUser {
        github_id: 90_000_002,
        login: "andras",
        avatar_url: "https://avatars.githubusercontent.com/u/5845193?v=4",
    },
    SeedUser {
        github_id: 90_000_003,
        login: "demo-builder",
        avatar_url: "https://avatars.githubusercontent.com/u/9919?v=4",
    },
    SeedUser {
        github_id: 90_000_004,
        login: "infra-friend",
        avatar_url: "https://avatars.githubusercontent.com/u/69631?v=4",
    },
];

const DEV_SEED_IDEAS: &[SeedIdea] = &[
    SeedIdea {
        title: "One-click status pages for Coolify services",
        body: "Generate public status pages from existing Coolify resources, including incidents, uptime history, and subscriber notifications.",
        problem: "Teams running Coolify have no built-in way to communicate outages. Hosted status pages like Statuspage are expensive and live outside the infrastructure they monitor.",
        author_login: "coollabs",
        upvoter_logins: &["andras", "demo-builder", "infra-friend"],
        closed: false,
    },
    SeedIdea {
        title: "Self-hosted changelog and release notes hub",
        body: "A small app for publishing product changelogs from Git tags, GitHub releases, and manually curated customer-facing updates.",
        problem: "Changelog SaaS tools are subscription-priced and lock content into their platform. Self-hosters want release notes that live next to their own app.",
        author_login: "andras",
        upvoter_logins: &["coollabs", "demo-builder"],
        closed: false,
    },
    SeedIdea {
        title: "Cron monitor with dead man switch alerts",
        body: "Track scheduled jobs by heartbeat URL and send alerts when backups, billing syncs, or maintenance tasks stop checking in.",
        problem: "Silent cron failures go unnoticed for days. Existing dead-man-switch services are hosted-only and bill per monitor, which adds up fast for self-hosters.",
        author_login: "infra-friend",
        upvoter_logins: &["coollabs", "andras", "demo-builder"],
        closed: false,
    },
    SeedIdea {
        title: "Tiny hosted forms backend for static sites",
        body: "Collect contact forms from static sites with spam controls, email forwarding, CSV export, and per-project API tokens.",
        problem: "Static sites cannot process form submissions on their own. Hosted form backends are priced per submission and keep customer data on third-party servers.",
        author_login: "demo-builder",
        upvoter_logins: &["andras"],
        closed: false,
    },
    SeedIdea {
        title: "Environment variable diff viewer for deployments",
        body: "Compare environment variables across staging and production without exposing secrets, highlighting missing keys and drift.",
        problem: "Config drift between environments causes hard-to-debug deploy failures. No lightweight tool diffs env vars without dumping secret values in plain text.",
        author_login: "andras",
        upvoter_logins: &["coollabs", "infra-friend"],
        closed: false,
    },
    SeedIdea {
        title: "Simple backup restore drill scheduler",
        body: "Schedule recurring restore drills, record evidence, and remind teams to prove their backups can actually be restored.",
        problem: "Backups are rarely tested until a real incident. No simple tool schedules restore drills and tracks evidence that recovery actually works.",
        author_login: "infra-friend",
        upvoter_logins: &["coollabs"],
        closed: true,
    },
];

#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
}

fn now_ts() -> i64 {
    Utc::now().timestamp()
}

fn parse_dt(value: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

impl Store {
    pub async fn connect(path: &str) -> anyhow::Result<Self> {
        let url = if path.starts_with("sqlite:") {
            path.to_string()
        } else {
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            format!("sqlite://{}", path)
        };
        let options = SqliteConnectOptions::from_str(&url)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(options)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        run_lenient_migrations(&self.pool).await?;
        Ok(())
    }

    pub async fn revert_latest(&self) -> anyhow::Result<()> {
        let applied: Vec<i64> = sqlx::query_scalar(
            "SELECT version FROM _sqlx_migrations WHERE success = 1 ORDER BY version",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        let target = if applied.len() >= 2 {
            applied[applied.len() - 2]
        } else {
            -1
        };
        embedded_migrator().undo(&self.pool, target).await?;
        Ok(())
    }

    pub async fn migration_versions(&self) -> anyhow::Result<Vec<i64>> {
        let rows = sqlx::query_scalar(
            "SELECT version FROM _sqlx_migrations WHERE success = 1 ORDER BY version",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        Ok(rows)
    }

    pub async fn seed_dev_examples(&self) -> Result<DevSeedReport> {
        let mut users = HashMap::new();
        for seed_user in DEV_SEED_USERS {
            let user = self
                .upsert_user(seed_user.github_id, seed_user.login, seed_user.avatar_url)
                .await?;
            users.insert(seed_user.login, user);
        }

        let mut ideas_by_title: HashMap<String, Idea> = self
            .list_ideas(None, false)
            .await?
            .into_iter()
            .map(|idea| (idea.title.clone(), idea))
            .collect();
        let mut seeded_idea_ids = HashSet::new();
        let mut seeded_upvotes = 0;

        for seed_idea in DEV_SEED_IDEAS {
            let author = users
                .get(seed_idea.author_login)
                .expect("seed idea author exists");
            let mut idea = if let Some(idea) = ideas_by_title.get(seed_idea.title).cloned() {
                idea
            } else {
                let idea = self
                    .create_idea(seed_idea.title, seed_idea.body, seed_idea.problem, author.id, false)
                    .await?;
                ideas_by_title.insert(seed_idea.title.to_string(), idea.clone());
                idea
            };

            if seed_idea.closed && !idea.closed {
                if let Ok(closed_idea) = self.set_idea_closed(idea.id, true, author.id, true).await
                {
                    idea = closed_idea;
                }
            }

            seeded_idea_ids.insert(idea.id);
            for login in seed_idea.upvoter_logins {
                let voter = users.get(login).expect("seed idea voter exists");
                self.set_upvote(idea.id, voter.id, true, false).await?;
                seeded_upvotes += 1;
            }
        }

        Ok(DevSeedReport {
            users: DEV_SEED_USERS.len(),
            ideas: seeded_idea_ids.len(),
            upvotes: seeded_upvotes,
        })
    }

    pub async fn sweep(&self) -> Result<()> {
        let now = now_ts();
        sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
            .bind(now)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM oauth_state WHERE created_at < ?")
            .bind(now - 600)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_oauth_state(&self, state: &str) -> Result<()> {
        sqlx::query("INSERT INTO oauth_state (state, created_at) VALUES (?, ?)")
            .bind(state)
            .bind(now_ts())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn consume_oauth_state(&self, state: &str) -> Result<bool> {
        let row = sqlx::query("SELECT created_at FROM oauth_state WHERE state = ?")
            .bind(state)
            .fetch_optional(&self.pool)
            .await?;
        sqlx::query("DELETE FROM oauth_state WHERE state = ?")
            .bind(state)
            .execute(&self.pool)
            .await?;
        Ok(row
            .and_then(|r| r.try_get::<i64, _>("created_at").ok())
            .is_some_and(|created_at| created_at >= now_ts() - 600))
    }

    pub async fn upsert_user(&self, github_id: i64, login: &str, avatar_url: &str) -> Result<User> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, github_id, login, avatar_url) VALUES (?, ?, ?, ?)\n             ON CONFLICT(github_id) DO UPDATE SET login = excluded.login, avatar_url = excluded.avatar_url, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        )
        .bind(id.to_string())
        .bind(github_id)
        .bind(login)
        .bind(avatar_url)
        .execute(&self.pool)
        .await?;
        self.user_by_github_id(github_id).await
    }

    pub async fn user_by_github_id(&self, github_id: i64) -> Result<User> {
        let row = sqlx::query("SELECT id, github_id, login, avatar_url, created_at, updated_at FROM users WHERE github_id = ?")
            .bind(github_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(User {
            id: Uuid::parse_str(row.try_get::<String, _>("id")?.as_str())
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            github_id: row.try_get("github_id")?,
            login: row.try_get("login")?,
            avatar_url: row.try_get("avatar_url")?,
            created_at: parse_dt(row.try_get("created_at")?),
            updated_at: parse_dt(row.try_get("updated_at")?),
        })
    }

    pub async fn create_session(
        &self,
        sid: &str,
        csrf: &str,
        user: &User,
        ttl_sec: i64,
    ) -> Result<Session> {
        let expires_at = now_ts() + ttl_sec;
        sqlx::query(
            "INSERT INTO sessions (sid, user_id, csrf_token, created_at, expires_at)\n             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(sid)
        .bind(user.id.to_string())
        .bind(csrf)
        .bind(now_ts())
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(Session {
            sid: sid.to_string(),
            user_id: user.id,
            csrf_token: csrf.to_string(),
            expires_at,
            user: user.clone(),
        })
    }

    pub async fn session(&self, sid: &str) -> Result<Option<Session>> {
        let row = sqlx::query(
            "SELECT s.sid, s.user_id, s.csrf_token, s.expires_at,\n                    u.id, u.github_id, u.login, u.avatar_url, u.created_at, u.updated_at\n             FROM sessions s JOIN users u ON u.id = s.user_id\n             WHERE s.sid = ? AND s.expires_at > ?",
        )
        .bind(sid)
        .bind(now_ts())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| {
            let uid = Uuid::parse_str(row.try_get::<String, _>("id")?.as_str())
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
            Ok(Session {
                sid: row.try_get("sid")?,
                user_id: uid,
                csrf_token: row.try_get("csrf_token")?,
                expires_at: row.try_get("expires_at")?,
                user: User {
                    id: uid,
                    github_id: row.try_get("github_id")?,
                    login: row.try_get("login")?,
                    avatar_url: row.try_get("avatar_url")?,
                    created_at: parse_dt(row.try_get("created_at")?),
                    updated_at: parse_dt(row.try_get("updated_at")?),
                },
            })
        })
        .transpose()
        .map_err(StorageError::Sqlx)
    }

    pub async fn delete_session(&self, sid: &str) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE sid = ?")
            .bind(sid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_idea(
        &self,
        title: &str,
        body: &str,
        problem: &str,
        author_id: Uuid,
        author_is_moderator: bool,
    ) -> Result<Idea> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ideas (id, title, body, problem, author_user_id, status) VALUES (?, ?, ?, ?, ?, 'open')")
            .bind(id.to_string())
            .bind(title)
            .bind(body)
            .bind(problem)
            .bind(author_id.to_string())
            .execute(&self.pool)
            .await?;
        self.idea(id, Some(author_id), author_is_moderator).await
    }

    pub async fn list_ideas(
        &self,
        viewer_id: Option<Uuid>,
        viewer_is_moderator: bool,
    ) -> Result<Vec<Idea>> {
        let viewer = viewer_id.map(|id| id.to_string());
        let moderator = i64::from(viewer_is_moderator);
        let comment_count_sql = self.comment_count_sql().await?;
        let sql = format!(
            "SELECT i.id, i.title, i.body, i.problem, i.status, i.created_at, i.updated_at,
                    u.login, u.avatar_url,
                    COUNT(v.idea_id) AS upvote_count,
                    {comment_count_sql} AS comment_count,
                    MAX(CASE WHEN (? IS NOT NULL AND v.user_id = ?) THEN 1 ELSE 0 END) AS viewer_has_upvoted,
                    CASE WHEN (? IS NOT NULL AND i.author_user_id = ?) THEN 1 ELSE 0 END AS viewer_can_edit,
                    CASE WHEN ((? IS NOT NULL AND i.author_user_id = ?) OR ? = 1) THEN 1 ELSE 0 END AS viewer_can_delete,
                    CASE WHEN ? = 1 THEN 1 ELSE 0 END AS viewer_can_close
             FROM ideas i
             JOIN users u ON u.id = i.author_user_id
             LEFT JOIN upvotes v ON v.idea_id = i.id
             GROUP BY i.id
             ORDER BY upvote_count DESC, i.created_at DESC"
        );
        let rows = sqlx::query(&sql)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(moderator)
            .bind(moderator)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter()
            .map(row_to_idea)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(StorageError::Sqlx)
    }

    pub async fn idea(
        &self,
        id: Uuid,
        viewer_id: Option<Uuid>,
        viewer_is_moderator: bool,
    ) -> Result<Idea> {
        let viewer = viewer_id.map(|id| id.to_string());
        let moderator = i64::from(viewer_is_moderator);
        let comment_count_sql = self.comment_count_sql().await?;
        let sql = format!(
            "SELECT i.id, i.title, i.body, i.problem, i.status, i.created_at, i.updated_at,
                    u.login, u.avatar_url,
                    COUNT(v.idea_id) AS upvote_count,
                    {comment_count_sql} AS comment_count,
                    MAX(CASE WHEN (? IS NOT NULL AND v.user_id = ?) THEN 1 ELSE 0 END) AS viewer_has_upvoted,
                    CASE WHEN (? IS NOT NULL AND i.author_user_id = ?) THEN 1 ELSE 0 END AS viewer_can_edit,
                    CASE WHEN ((? IS NOT NULL AND i.author_user_id = ?) OR ? = 1) THEN 1 ELSE 0 END AS viewer_can_delete,
                    CASE WHEN ? = 1 THEN 1 ELSE 0 END AS viewer_can_close
             FROM ideas i
             JOIN users u ON u.id = i.author_user_id
             LEFT JOIN upvotes v ON v.idea_id = i.id
             WHERE i.id = ?
             GROUP BY i.id"
        );
        let row = sqlx::query(&sql)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(moderator)
            .bind(moderator)
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_idea)
            .transpose()
            .map_err(StorageError::Sqlx)?
            .ok_or(StorageError::NotFound("idea"))
    }

    pub async fn update_idea(
        &self,
        id: Uuid,
        title: &str,
        body: &str,
        problem: &str,
        actor_id: Uuid,
        actor_is_moderator: bool,
    ) -> Result<Idea> {
        let changed = sqlx::query("UPDATE ideas SET title = ?, body = ?, problem = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ? AND author_user_id = ?")
            .bind(title)
            .bind(body)
            .bind(problem)
            .bind(id.to_string())
            .bind(actor_id.to_string())
            .execute(&self.pool)
            .await?
            .rows_affected();
        if changed == 0 {
            return Err(StorageError::NotFound("idea"));
        }
        self.idea(id, Some(actor_id), actor_is_moderator).await
    }

    pub async fn set_idea_status(
        &self,
        id: Uuid,
        status: IdeaStatus,
        actor_id: Uuid,
        actor_is_moderator: bool,
    ) -> Result<Idea> {
        if !actor_is_moderator {
            return Err(StorageError::NotFound("idea"));
        }
        let changed = sqlx::query("UPDATE ideas SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?")
            .bind(status.as_str())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?
            .rows_affected();
        if changed == 0 {
            return Err(StorageError::NotFound("idea"));
        }
        self.idea(id, Some(actor_id), actor_is_moderator).await
    }

    pub async fn set_idea_closed(
        &self,
        id: Uuid,
        closed: bool,
        actor_id: Uuid,
        actor_is_moderator: bool,
    ) -> Result<Idea> {
        let status = if closed {
            IdeaStatus::Closed
        } else {
            IdeaStatus::Open
        };
        self.set_idea_status(id, status, actor_id, actor_is_moderator)
            .await
    }

    pub async fn delete_idea(
        &self,
        id: Uuid,
        actor_id: Uuid,
        actor_is_moderator: bool,
    ) -> Result<()> {
        let changed = if actor_is_moderator {
            sqlx::query("DELETE FROM ideas WHERE id = ?")
                .bind(id.to_string())
                .execute(&self.pool)
                .await?
                .rows_affected()
        } else {
            sqlx::query("DELETE FROM ideas WHERE id = ? AND author_user_id = ?")
                .bind(id.to_string())
                .bind(actor_id.to_string())
                .execute(&self.pool)
                .await?
                .rows_affected()
        };
        if changed == 0 {
            return Err(StorageError::NotFound("idea"));
        }
        Ok(())
    }

    pub async fn set_upvote(
        &self,
        idea_id: Uuid,
        user_id: Uuid,
        upvoted: bool,
        viewer_is_moderator: bool,
    ) -> Result<Idea> {
        if upvoted {
            sqlx::query("INSERT OR IGNORE INTO upvotes (idea_id, user_id) VALUES (?, ?)")
                .bind(idea_id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("DELETE FROM upvotes WHERE idea_id = ? AND user_id = ?")
                .bind(idea_id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }
        self.idea(idea_id, Some(user_id), viewer_is_moderator).await
    }

    pub async fn set_comment_upvote(
        &self,
        comment_id: Uuid,
        user_id: Uuid,
        upvoted: bool,
        viewer_is_moderator: bool,
    ) -> Result<Comment> {
        if !self.comment_exists(comment_id).await? {
            return Err(StorageError::NotFound("comment"));
        }

        if upvoted {
            sqlx::query(
                "INSERT OR IGNORE INTO comment_upvotes (comment_id, user_id) VALUES (?, ?)",
            )
            .bind(comment_id.to_string())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query("DELETE FROM comment_upvotes WHERE comment_id = ? AND user_id = ?")
                .bind(comment_id.to_string())
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await?;
        }
        self.comment(comment_id, Some(user_id), viewer_is_moderator)
            .await
    }

    pub async fn list_comments(
        &self,
        idea_id: Uuid,
        viewer_id: Option<Uuid>,
        viewer_is_moderator: bool,
    ) -> Result<Vec<Comment>> {
        if !self.idea_exists(idea_id).await? {
            return Err(StorageError::NotFound("idea"));
        }
        let viewer = viewer_id.map(|id| id.to_string());
        let moderator = i64::from(viewer_is_moderator);
        let (comment_upvote_select_sql, comment_upvote_join_sql) =
            self.comment_upvote_sql().await?;
        let sql = format!(
            "SELECT c.id, c.idea_id, c.body, c.created_at, c.updated_at,
                    u.login, u.avatar_url,
                    {comment_upvote_select_sql},
                    CASE WHEN (? IS NOT NULL AND c.author_user_id = ?) THEN 1 ELSE 0 END AS viewer_can_edit,
                    CASE WHEN ((? IS NOT NULL AND c.author_user_id = ?) OR ? = 1) THEN 1 ELSE 0 END AS viewer_can_delete
             FROM comments c
             JOIN users u ON u.id = c.author_user_id
             {comment_upvote_join_sql}
             WHERE c.idea_id = ?
             GROUP BY c.id
             ORDER BY c.created_at ASC"
        );
        let rows = sqlx::query(&sql)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(moderator)
            .bind(idea_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        let mut comments = rows
            .into_iter()
            .map(row_to_comment)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(StorageError::Sqlx)?;
        promote_most_upvoted_comment(&mut comments);
        Ok(comments)
    }

    pub async fn create_comment(
        &self,
        idea_id: Uuid,
        body: &str,
        author_id: Uuid,
        author_is_moderator: bool,
    ) -> Result<Comment> {
        if !self.idea_exists(idea_id).await? {
            return Err(StorageError::NotFound("idea"));
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO comments (id, idea_id, author_user_id, body) VALUES (?, ?, ?, ?)")
            .bind(id.to_string())
            .bind(idea_id.to_string())
            .bind(author_id.to_string())
            .bind(body)
            .execute(&self.pool)
            .await?;
        self.comment(id, Some(author_id), author_is_moderator).await
    }

    pub async fn update_comment(
        &self,
        id: Uuid,
        body: &str,
        actor_id: Uuid,
        actor_is_moderator: bool,
    ) -> Result<Comment> {
        let changed = sqlx::query("UPDATE comments SET body = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ? AND author_user_id = ?")
            .bind(body)
            .bind(id.to_string())
            .bind(actor_id.to_string())
            .execute(&self.pool)
            .await?
            .rows_affected();
        if changed == 0 {
            return Err(StorageError::NotFound("comment"));
        }
        self.comment(id, Some(actor_id), actor_is_moderator).await
    }

    pub async fn delete_comment(
        &self,
        id: Uuid,
        actor_id: Uuid,
        actor_is_moderator: bool,
    ) -> Result<()> {
        let changed = if actor_is_moderator {
            sqlx::query("DELETE FROM comments WHERE id = ?")
                .bind(id.to_string())
                .execute(&self.pool)
                .await?
                .rows_affected()
        } else {
            sqlx::query("DELETE FROM comments WHERE id = ? AND author_user_id = ?")
                .bind(id.to_string())
                .bind(actor_id.to_string())
                .execute(&self.pool)
                .await?
                .rows_affected()
        };
        if changed == 0 {
            return Err(StorageError::NotFound("comment"));
        }
        Ok(())
    }

    async fn comment(
        &self,
        id: Uuid,
        viewer_id: Option<Uuid>,
        viewer_is_moderator: bool,
    ) -> Result<Comment> {
        let viewer = viewer_id.map(|id| id.to_string());
        let moderator = i64::from(viewer_is_moderator);
        let (comment_upvote_select_sql, comment_upvote_join_sql) =
            self.comment_upvote_sql().await?;
        let sql = format!(
            "SELECT c.id, c.idea_id, c.body, c.created_at, c.updated_at,
                    u.login, u.avatar_url,
                    {comment_upvote_select_sql},
                    CASE WHEN (? IS NOT NULL AND c.author_user_id = ?) THEN 1 ELSE 0 END AS viewer_can_edit,
                    CASE WHEN ((? IS NOT NULL AND c.author_user_id = ?) OR ? = 1) THEN 1 ELSE 0 END AS viewer_can_delete
             FROM comments c
             JOIN users u ON u.id = c.author_user_id
             {comment_upvote_join_sql}
             WHERE c.id = ?
             GROUP BY c.id"
        );
        let row = sqlx::query(&sql)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(&viewer)
            .bind(moderator)
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_comment)
            .transpose()
            .map_err(StorageError::Sqlx)?
            .ok_or(StorageError::NotFound("comment"))
    }

    async fn comment_count_sql(&self) -> Result<&'static str> {
        if self.comments_table_exists().await? {
            Ok("(SELECT COUNT(*) FROM comments c WHERE c.idea_id = i.id)")
        } else {
            Ok("0")
        }
    }

    async fn comment_upvote_sql(&self) -> Result<(&'static str, &'static str)> {
        if self.comment_upvotes_table_exists().await? {
            Ok((
                "COUNT(cu.comment_id) AS upvote_count, MAX(CASE WHEN (? IS NOT NULL AND cu.user_id = ?) THEN 1 ELSE 0 END) AS viewer_has_upvoted",
                "LEFT JOIN comment_upvotes cu ON cu.comment_id = c.id",
            ))
        } else {
            Ok((
                "CASE WHEN (? IS NOT NULL AND ? IS NOT NULL) THEN 0 ELSE 0 END AS upvote_count, 0 AS viewer_has_upvoted",
                "",
            ))
        }
    }

    async fn comments_table_exists(&self) -> Result<bool> {
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'comments'",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(exists.is_some())
    }

    async fn comment_upvotes_table_exists(&self) -> Result<bool> {
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'comment_upvotes'",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(exists.is_some())
    }

    async fn idea_exists(&self, id: Uuid) -> Result<bool> {
        let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM ideas WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(exists.is_some())
    }

    async fn comment_exists(&self, id: Uuid) -> Result<bool> {
        let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM comments WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(exists.is_some())
    }
}

fn row_to_idea(row: sqlx::sqlite::SqliteRow) -> std::result::Result<Idea, sqlx::Error> {
    let id = Uuid::parse_str(row.try_get::<String, _>("id")?.as_str())
        .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
    let status = IdeaStatus::from_db(&row.try_get::<String, _>("status")?);
    Ok(Idea {
        id,
        title: row.try_get("title")?,
        body_text: row.try_get("body")?,
        problem: row.try_get("problem")?,
        upvote_count: row.try_get("upvote_count")?,
        comment_count: row.try_get("comment_count")?,
        viewer_has_upvoted: row.try_get::<i64, _>("viewer_has_upvoted")? == 1,
        viewer_can_edit: row.try_get::<i64, _>("viewer_can_edit")? == 1,
        viewer_can_delete: row.try_get::<i64, _>("viewer_can_delete")? == 1,
        viewer_can_close: row.try_get::<i64, _>("viewer_can_close")? == 1,
        author: PublicUser {
            login: row.try_get("login")?,
            avatar_url: row.try_get("avatar_url")?,
        },
        created_at: parse_dt(row.try_get("created_at")?),
        updated_at: parse_dt(row.try_get("updated_at")?),
        status,
        closed: status == IdeaStatus::Closed,
    })
}

fn row_to_comment(row: sqlx::sqlite::SqliteRow) -> std::result::Result<Comment, sqlx::Error> {
    Ok(Comment {
        id: Uuid::parse_str(row.try_get::<String, _>("id")?.as_str())
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        idea_id: Uuid::parse_str(row.try_get::<String, _>("idea_id")?.as_str())
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        body_text: row.try_get("body")?,
        upvote_count: row.try_get("upvote_count")?,
        viewer_has_upvoted: row.try_get::<i64, _>("viewer_has_upvoted")? == 1,
        viewer_can_edit: row.try_get::<i64, _>("viewer_can_edit")? == 1,
        viewer_can_delete: row.try_get::<i64, _>("viewer_can_delete")? == 1,
        author: PublicUser {
            login: row.try_get("login")?,
            avatar_url: row.try_get("avatar_url")?,
        },
        created_at: parse_dt(row.try_get("created_at")?),
        updated_at: parse_dt(row.try_get("updated_at")?),
    })
}

fn promote_most_upvoted_comment(comments: &mut Vec<Comment>) {
    let Some((top_index, top_upvotes)) = comments
        .iter()
        .enumerate()
        .filter(|(_, comment)| comment.upvote_count > 0)
        .fold(None, |best, (index, comment)| match best {
            Some((_, best_upvotes)) if best_upvotes >= comment.upvote_count => best,
            _ => Some((index, comment.upvote_count)),
        })
    else {
        return;
    };

    if top_upvotes == 0 || top_index == 0 {
        return;
    }

    let top_comment = comments.remove(top_index);
    comments.insert(0, top_comment);
}

#[cfg(test)]
mod tests {
    use super::{IdeaStatus, StorageError, Store};
    use uuid::Uuid;

    async fn test_store() -> Store {
        let path = std::env::temp_dir().join(format!("ideas-storage-test-{}.db", Uuid::new_v4()));
        let store = Store::connect(path.to_str().expect("utf8 temp path"))
            .await
            .expect("connect");
        store.migrate().await.expect("migrate");
        store
    }

    #[tokio::test]
    async fn regular_author_can_edit_and_delete_but_cannot_close_own_idea() {
        let store = test_store().await;
        let author = store
            .upsert_user(1, "author", "https://example.com/author.png")
            .await
            .expect("author");
        let idea = store
            .create_idea(
                "A regular author idea",
                "This is long enough body text for a valid idea.",
                "This problem statement is long enough to pass validation.",
                author.id,
                false,
            )
            .await
            .expect("idea");

        assert!(idea.viewer_can_edit);
        assert!(idea.viewer_can_delete);
        assert!(!idea.viewer_can_close);

        let updated = store
            .update_idea(
                idea.id,
                "A changed author idea",
                "This updated body text remains long enough for a valid idea.",
                "This updated problem statement is long enough to pass validation.",
                author.id,
                false,
            )
            .await
            .expect("update own idea");
        assert_eq!(updated.title, "A changed author idea");

        let close_error = store
            .set_idea_closed(idea.id, true, author.id, false)
            .await
            .expect_err("regular author cannot close own idea");
        assert!(matches!(close_error, StorageError::NotFound("idea")));

        store
            .delete_idea(idea.id, author.id, false)
            .await
            .expect("delete own idea");
    }

    #[tokio::test]
    async fn moderator_can_close_and_delete_someone_elses_idea() {
        let store = test_store().await;
        let author = store
            .upsert_user(2, "author", "https://example.com/author.png")
            .await
            .expect("author");
        let moderator = store
            .upsert_user(3, "moderator", "https://example.com/moderator.png")
            .await
            .expect("moderator");
        let idea = store
            .create_idea(
                "A moderator managed idea",
                "This is long enough body text for a valid idea.",
                "This problem statement is long enough to pass validation.",
                author.id,
                false,
            )
            .await
            .expect("idea");

        let viewed = store
            .idea(idea.id, Some(moderator.id), true)
            .await
            .expect("moderator view");
        assert!(!viewed.viewer_can_edit);
        assert!(viewed.viewer_can_delete);
        assert!(viewed.viewer_can_close);

        let closed = store
            .set_idea_closed(idea.id, true, moderator.id, true)
            .await
            .expect("moderator close");
        assert!(closed.closed);
        assert!(closed.viewer_can_close);

        store
            .delete_idea(idea.id, moderator.id, true)
            .await
            .expect("moderator delete");
    }

    #[tokio::test]
    async fn moderator_can_mark_idea_in_progress() {
        let store = test_store().await;
        let author = store
            .upsert_user(6, "author", "https://example.com/author.png")
            .await
            .expect("author");
        let moderator = store
            .upsert_user(7, "moderator", "https://example.com/moderator.png")
            .await
            .expect("moderator");
        let idea = store
            .create_idea(
                "An idea ready for progress",
                "This is long enough body text for a valid idea.",
                "This problem statement is long enough to pass validation.",
                author.id,
                false,
            )
            .await
            .expect("idea");

        let in_progress = store
            .set_idea_status(idea.id, IdeaStatus::InProgress, moderator.id, true)
            .await
            .expect("moderator mark in progress");
        assert_eq!(in_progress.status, IdeaStatus::InProgress);
        assert!(!in_progress.closed);

        let public_view = store.idea(idea.id, None, false).await.expect("public view");
        assert_eq!(public_view.status, IdeaStatus::InProgress);
        assert!(!public_view.closed);
    }

    #[tokio::test]
    async fn regular_author_cannot_mark_idea_in_progress() {
        let store = test_store().await;
        let author = store
            .upsert_user(8, "author", "https://example.com/author.png")
            .await
            .expect("author");
        let idea = store
            .create_idea(
                "A regular author idea",
                "This is long enough body text for a valid idea.",
                "This problem statement is long enough to pass validation.",
                author.id,
                false,
            )
            .await
            .expect("idea");

        let error = store
            .set_idea_status(idea.id, IdeaStatus::InProgress, author.id, false)
            .await
            .expect_err("regular author cannot mark in progress");
        assert!(matches!(error, StorageError::NotFound("idea")));
    }

    #[tokio::test]
    async fn comments_are_listed_with_author_permissions_and_counts() {
        let store = test_store().await;
        let author = store
            .upsert_user(9, "author", "https://example.com/author.png")
            .await
            .expect("author");
        let commenter = store
            .upsert_user(10, "commenter", "https://example.com/commenter.png")
            .await
            .expect("commenter");
        let idea = store
            .create_idea(
                "A commentable idea",
                "This is long enough body text for a valid idea.",
                "This problem statement is long enough to pass validation.",
                author.id,
                false,
            )
            .await
            .expect("idea");

        let comment = store
            .create_comment(idea.id, "I would use this.", commenter.id, false)
            .await
            .expect("comment");
        assert_eq!(comment.body_text, "I would use this.");
        assert!(comment.viewer_can_edit);
        assert!(comment.viewer_can_delete);

        let comments = store
            .list_comments(idea.id, Some(author.id), false)
            .await
            .expect("comments");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].author.login, "commenter");
        assert!(!comments[0].viewer_can_edit);
        assert!(!comments[0].viewer_can_delete);

        let updated_idea = store
            .idea(idea.id, Some(author.id), false)
            .await
            .expect("idea");
        assert_eq!(updated_idea.comment_count, 1);
    }

    #[tokio::test]
    async fn comment_author_can_update_and_moderator_can_delete() {
        let store = test_store().await;
        let author = store
            .upsert_user(11, "author", "https://example.com/author.png")
            .await
            .expect("author");
        let commenter = store
            .upsert_user(12, "commenter", "https://example.com/commenter.png")
            .await
            .expect("commenter");
        let other = store
            .upsert_user(13, "other", "https://example.com/other.png")
            .await
            .expect("other");
        let moderator = store
            .upsert_user(14, "moderator", "https://example.com/moderator.png")
            .await
            .expect("moderator");
        let idea = store
            .create_idea(
                "Another commentable idea",
                "This is long enough body text for a valid idea.",
                "This problem statement is long enough to pass validation.",
                author.id,
                false,
            )
            .await
            .expect("idea");
        let comment = store
            .create_comment(idea.id, "Original note", commenter.id, false)
            .await
            .expect("comment");

        let update_error = store
            .update_comment(comment.id, "Not allowed", other.id, false)
            .await
            .expect_err("other cannot update");
        assert!(matches!(update_error, StorageError::NotFound("comment")));

        let updated = store
            .update_comment(comment.id, "Updated note", commenter.id, false)
            .await
            .expect("author updates");
        assert_eq!(updated.body_text, "Updated note");

        store
            .delete_comment(comment.id, moderator.id, true)
            .await
            .expect("moderator deletes");
        let comments = store
            .list_comments(idea.id, Some(moderator.id), true)
            .await
            .expect("comments");
        assert!(comments.is_empty());
    }

    #[tokio::test]
    async fn non_author_cannot_delete_someone_elses_idea() {
        let store = test_store().await;
        let author = store
            .upsert_user(4, "author", "https://example.com/author.png")
            .await
            .expect("author");
        let other = store
            .upsert_user(5, "other", "https://example.com/other.png")
            .await
            .expect("other");
        let idea = store
            .create_idea(
                "Another user's idea",
                "This is long enough body text for a valid idea.",
                "This problem statement is long enough to pass validation.",
                author.id,
                false,
            )
            .await
            .expect("idea");

        let viewed = store
            .idea(idea.id, Some(other.id), false)
            .await
            .expect("other view");
        assert!(!viewed.viewer_can_edit);
        assert!(!viewed.viewer_can_delete);
        assert!(!viewed.viewer_can_close);

        let delete_error = store
            .delete_idea(idea.id, other.id, false)
            .await
            .expect_err("non-author cannot delete");
        assert!(matches!(delete_error, StorageError::NotFound("idea")));
    }
}
