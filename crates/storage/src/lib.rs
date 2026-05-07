use chrono::{DateTime, Utc};
use ideas_domain::{Idea, PublicUser, Session, User};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

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
        MIGRATOR.run(&self.pool).await?;
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
        MIGRATOR.undo(&self.pool, target).await?;
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

    pub async fn create_idea(&self, title: &str, body: &str, author_id: Uuid) -> Result<Idea> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ideas (id, title, body, author_user_id, status) VALUES (?, ?, ?, ?, 'open')")
            .bind(id.to_string())
            .bind(title)
            .bind(body)
            .bind(author_id.to_string())
            .execute(&self.pool)
            .await?;
        self.idea(id, Some(author_id)).await
    }

    pub async fn list_ideas(&self, viewer_id: Option<Uuid>) -> Result<Vec<Idea>> {
        let viewer = viewer_id.map(|id| id.to_string());
        let rows = sqlx::query(
            "SELECT i.id, i.title, i.body, i.status, i.created_at, i.updated_at,\n                    u.login, u.avatar_url,\n                    COUNT(v.idea_id) AS upvote_count,\n                    MAX(CASE WHEN (? IS NOT NULL AND v.user_id = ?) THEN 1 ELSE 0 END) AS viewer_has_upvoted\n             FROM ideas i\n             JOIN users u ON u.id = i.author_user_id\n             LEFT JOIN upvotes v ON v.idea_id = i.id\n             GROUP BY i.id\n             ORDER BY upvote_count DESC, i.created_at DESC",
        )
        .bind(&viewer)
        .bind(&viewer)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(row_to_idea)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(StorageError::Sqlx)
    }

    pub async fn idea(&self, id: Uuid, viewer_id: Option<Uuid>) -> Result<Idea> {
        let viewer = viewer_id.map(|id| id.to_string());
        let row = sqlx::query(
            "SELECT i.id, i.title, i.body, i.status, i.created_at, i.updated_at,\n                    u.login, u.avatar_url,\n                    COUNT(v.idea_id) AS upvote_count,\n                    MAX(CASE WHEN (? IS NOT NULL AND v.user_id = ?) THEN 1 ELSE 0 END) AS viewer_has_upvoted\n             FROM ideas i\n             JOIN users u ON u.id = i.author_user_id\n             LEFT JOIN upvotes v ON v.idea_id = i.id\n             WHERE i.id = ?\n             GROUP BY i.id",
        )
        .bind(&viewer)
        .bind(&viewer)
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
        actor_id: Uuid,
    ) -> Result<Idea> {
        let changed = sqlx::query("UPDATE ideas SET title = ?, body = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ? AND author_user_id = ?")
            .bind(title)
            .bind(body)
            .bind(id.to_string())
            .bind(actor_id.to_string())
            .execute(&self.pool)
            .await?
            .rows_affected();
        if changed == 0 {
            return Err(StorageError::NotFound("idea"));
        }
        self.idea(id, Some(actor_id)).await
    }

    pub async fn set_idea_closed(&self, id: Uuid, closed: bool, actor_id: Uuid) -> Result<Idea> {
        let status = if closed { "closed" } else { "open" };
        let changed = sqlx::query("UPDATE ideas SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ? AND author_user_id = ?")
            .bind(status)
            .bind(id.to_string())
            .bind(actor_id.to_string())
            .execute(&self.pool)
            .await?
            .rows_affected();
        if changed == 0 {
            return Err(StorageError::NotFound("idea"));
        }
        self.idea(id, Some(actor_id)).await
    }

    pub async fn delete_idea(&self, id: Uuid, actor_id: Uuid) -> Result<()> {
        let changed = sqlx::query("DELETE FROM ideas WHERE id = ? AND author_user_id = ?")
            .bind(id.to_string())
            .bind(actor_id.to_string())
            .execute(&self.pool)
            .await?
            .rows_affected();
        if changed == 0 {
            return Err(StorageError::NotFound("idea"));
        }
        Ok(())
    }

    pub async fn set_upvote(&self, idea_id: Uuid, user_id: Uuid, upvoted: bool) -> Result<Idea> {
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
        self.idea(idea_id, Some(user_id)).await
    }
}

fn row_to_idea(row: sqlx::sqlite::SqliteRow) -> std::result::Result<Idea, sqlx::Error> {
    let id = Uuid::parse_str(row.try_get::<String, _>("id")?.as_str())
        .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
    let status: String = row.try_get("status")?;
    Ok(Idea {
        id,
        title: row.try_get("title")?,
        body_text: row.try_get("body")?,
        upvote_count: row.try_get("upvote_count")?,
        viewer_has_upvoted: row.try_get::<i64, _>("viewer_has_upvoted")? == 1,
        author: PublicUser {
            login: row.try_get("login")?,
            avatar_url: row.try_get("avatar_url")?,
        },
        created_at: parse_dt(row.try_get("created_at")?),
        updated_at: parse_dt(row.try_get("updated_at")?),
        closed: status == "closed",
    })
}
