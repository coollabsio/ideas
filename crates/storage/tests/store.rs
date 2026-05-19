use ideas_domain::IdeaStatus;
use ideas_storage::Store;
use sqlx::migrate::{Migration, MigrationType, Migrator};
use std::borrow::Cow;
use uuid::Uuid;

const INIT_UP_SQL: &str = include_str!("../migrations/20260507120000_init.up.sql");
const ADD_INPROGRESS_STATUS_UP_SQL: &str =
    include_str!("../migrations/20260518120000_add_inprogress_status.up.sql");
const ADD_COMMENTS_UP_SQL: &str = include_str!("../migrations/20260518130000_add_comments.up.sql");
const ADD_IDEA_PROBLEM_UP_SQL: &str =
    include_str!("../migrations/20260519120000_add_idea_problem.up.sql");

const ORIGINAL_ADD_INPROGRESS_STATUS_UP_SQL: &str = r#"-- no-transaction
PRAGMA foreign_keys = OFF;

CREATE TABLE ideas_new (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL CHECK(length(title) BETWEEN 10 AND 300),
  body TEXT NOT NULL CHECK(length(body) BETWEEN 30 AND 10000),
  author_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open', 'inprogress', 'closed')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

INSERT INTO ideas_new (id, title, body, author_user_id, status, created_at, updated_at)
SELECT id, title, body, author_user_id, status, created_at, updated_at
FROM ideas;

DROP TABLE ideas;
ALTER TABLE ideas_new RENAME TO ideas;

CREATE INDEX idx_ideas_status_created ON ideas(status, created_at DESC);
CREATE INDEX idx_ideas_author ON ideas(author_user_id);

PRAGMA foreign_keys = ON;
"#;

async fn test_store() -> Store {
    let path = std::env::temp_dir().join(format!("ideas-test-{}.db", Uuid::new_v4()));
    let store = Store::connect(path.to_str().expect("utf8 temp path"))
        .await
        .expect("connect");
    store.migrate().await.expect("migrate");
    store
}

#[tokio::test]
async fn status_migration_preserves_upvotes_when_drop_cascades() {
    let path = std::env::temp_dir().join(format!("ideas-migration-test-{}.db", Uuid::new_v4()));
    let store = Store::connect(path.to_str().expect("utf8 temp path"))
        .await
        .expect("connect");
    sqlx::raw_sql(INIT_UP_SQL)
        .execute(store.pool())
        .await
        .expect("init schema");
    // `Store::idea` (used by `set_upvote`) selects the `problem` column, so the
    // column must exist before any store call, even pre-`add_inprogress_status`.
    sqlx::raw_sql(ADD_IDEA_PROBLEM_UP_SQL)
        .execute(store.pool())
        .await
        .expect("add idea problem column");

    let author = store
        .upsert_user(101, "author", "https://example.com/author.png")
        .await
        .expect("author");
    let voter = store
        .upsert_user(102, "voter", "https://example.com/voter.png")
        .await
        .expect("voter");
    // Insert the idea under the original (pre-`problem`) schema, mirroring real
    // pre-migration data this test is meant to exercise.
    let idea_id = Uuid::new_v4();
    sqlx::query("INSERT INTO ideas (id, title, body, author_user_id, status) VALUES (?, ?, ?, ?, 'open')")
        .bind(idea_id.to_string())
        .bind("A migration safety idea")
        .bind("This body is long enough to satisfy validation before migration.")
        .bind(author.id.to_string())
        .execute(store.pool())
        .await
        .expect("insert idea");
    store
        .set_upvote(idea_id, voter.id, true, false)
        .await
        .expect("upvote before migration");

    let migration = ADD_INPROGRESS_STATUS_UP_SQL
        .replace("PRAGMA foreign_keys = OFF;", "PRAGMA foreign_keys = ON;");
    sqlx::raw_sql(&migration)
        .execute(store.pool())
        .await
        .expect("status migration");
    // The status migration recreates the `ideas` table; re-add `problem` if the
    // recreated table dropped it, so `Store::idea` can select the column.
    let has_problem: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM pragma_table_info('ideas') WHERE name = 'problem'")
            .fetch_optional(store.pool())
            .await
            .expect("check problem column");
    if has_problem.is_none() {
        sqlx::raw_sql(ADD_IDEA_PROBLEM_UP_SQL)
            .execute(store.pool())
            .await
            .expect("add idea problem column");
    }

    let migrated = store
        .idea(idea_id, Some(voter.id), false)
        .await
        .expect("migrated idea");
    assert_eq!(migrated.upvote_count, 1);
    assert!(migrated.viewer_has_upvoted);

    let upvote_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM upvotes")
        .fetch_one(store.pool())
        .await
        .expect("upvote row count");
    assert_eq!(upvote_rows, 1);
}

#[tokio::test]
async fn migrate_skips_applied_migration_with_checksum_mismatch_and_applies_pending() {
    let path = std::env::temp_dir().join(format!("ideas-checksum-test-{}.db", Uuid::new_v4()));
    let store = Store::connect(path.to_str().expect("utf8 temp path"))
        .await
        .expect("connect");
    let old_migrator = Migrator {
        migrations: Cow::Owned(vec![
            Migration::new(
                20260507120000,
                Cow::Borrowed("init"),
                MigrationType::ReversibleUp,
                Cow::Borrowed(INIT_UP_SQL),
                INIT_UP_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260518120000,
                Cow::Borrowed("add_inprogress_status"),
                MigrationType::ReversibleUp,
                Cow::Borrowed(ORIGINAL_ADD_INPROGRESS_STATUS_UP_SQL),
                ORIGINAL_ADD_INPROGRESS_STATUS_UP_SQL.starts_with("-- no-transaction"),
            ),
        ]),
        ..Migrator::DEFAULT
    };
    old_migrator
        .run(store.pool())
        .await
        .expect("old migrations apply");

    store.migrate().await.expect("checksum mismatch is skipped");

    let comments_table: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'comments'",
    )
    .fetch_optional(store.pool())
    .await
    .expect("comments table lookup");
    assert_eq!(comments_table.as_deref(), Some("comments"));

    let comment_upvotes_table: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'comment_upvotes'",
    )
    .fetch_optional(store.pool())
    .await
    .expect("comment upvotes table lookup");
    assert_eq!(comment_upvotes_table.as_deref(), Some("comment_upvotes"));
}

#[tokio::test]
async fn migrate_still_fails_on_dirty_migration() {
    let path = std::env::temp_dir().join(format!("ideas-dirty-test-{}.db", Uuid::new_v4()));
    let store = Store::connect(path.to_str().expect("utf8 temp path"))
        .await
        .expect("connect");

    sqlx::raw_sql(
        r#"
        CREATE TABLE _sqlx_migrations (
            version BIGINT PRIMARY KEY,
            description TEXT NOT NULL,
            installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            success BOOLEAN NOT NULL,
            checksum BLOB NOT NULL,
            execution_time BIGINT NOT NULL
        );
        INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
        VALUES (20260518129999, 'dirty', FALSE, X'00', 0);
        "#,
    )
    .execute(store.pool())
    .await
    .expect("dirty migration row");

    let err = store.migrate().await.expect_err("dirty migration fails");
    assert!(
        err.to_string().contains("partially applied"),
        "unexpected error: {err:#}"
    );
}

#[tokio::test]
async fn creates_and_toggles_upvotes() {
    let store = test_store().await;
    let user = store
        .upsert_user(1, "alice", "https://example.com/a.png")
        .await
        .expect("user");
    let other = store
        .upsert_user(2, "bob", "https://example.com/b.png")
        .await
        .expect("other user");

    let idea = store
        .create_idea(
            "A useful local idea",
            "This body is long enough to satisfy validation.",
            "This problem statement is long enough to satisfy validation.",
            user.id,
            false,
        )
        .await
        .expect("idea");
    assert_eq!(idea.upvote_count, 0);

    let voted = store
        .set_upvote(idea.id, other.id, true, false)
        .await
        .expect("vote");
    assert_eq!(voted.upvote_count, 1);
    assert!(voted.viewer_has_upvoted);

    let voted_again = store
        .set_upvote(idea.id, other.id, true, false)
        .await
        .expect("idempotent vote");
    assert_eq!(voted_again.upvote_count, 1);

    let unvoted = store
        .set_upvote(idea.id, other.id, false, false)
        .await
        .expect("unvote");
    assert_eq!(unvoted.upvote_count, 0);
    assert!(!unvoted.viewer_has_upvoted);
}

#[tokio::test]
async fn comment_upvote_migration_preserves_existing_comments() {
    let path = std::env::temp_dir().join(format!(
        "ideas-comment-migration-test-{}.db",
        Uuid::new_v4()
    ));
    let store = Store::connect(path.to_str().expect("utf8 temp path"))
        .await
        .expect("connect");
    let old_migrator = Migrator {
        migrations: Cow::Owned(vec![
            Migration::new(
                20260507120000,
                Cow::Borrowed("init"),
                MigrationType::ReversibleUp,
                Cow::Borrowed(INIT_UP_SQL),
                INIT_UP_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260518120000,
                Cow::Borrowed("add_inprogress_status"),
                MigrationType::ReversibleUp,
                Cow::Borrowed(ADD_INPROGRESS_STATUS_UP_SQL),
                ADD_INPROGRESS_STATUS_UP_SQL.starts_with("-- no-transaction"),
            ),
            Migration::new(
                20260518130000,
                Cow::Borrowed("add_comments"),
                MigrationType::ReversibleUp,
                Cow::Borrowed(ADD_COMMENTS_UP_SQL),
                ADD_COMMENTS_UP_SQL.starts_with("-- no-transaction"),
            ),
        ]),
        ..Migrator::DEFAULT
    };
    old_migrator
        .run(store.pool())
        .await
        .expect("old migrations apply");

    let author = store
        .upsert_user(201, "author", "https://example.com/author.png")
        .await
        .expect("author");
    // Insert the idea under the old schema (before the `problem` column existed),
    // since this test exercises pre-migration data.
    let idea_id = Uuid::new_v4();
    sqlx::query("INSERT INTO ideas (id, title, body, author_user_id, status) VALUES (?, ?, ?, ?, 'open')")
        .bind(idea_id.to_string())
        .bind("A comment migration idea")
        .bind("This body is long enough for a migration preservation test.")
        .bind(author.id.to_string())
        .execute(store.pool())
        .await
        .expect("insert idea");
    let comment = store
        .create_comment(idea_id, "Keep this existing comment.", author.id, false)
        .await
        .expect("comment before migration");

    store.migrate().await.expect("comment upvote migration");

    let comments = store
        .list_comments(idea_id, Some(author.id), false)
        .await
        .expect("comments after migration");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].id, comment.id);
    assert_eq!(comments[0].body_text, "Keep this existing comment.");
    assert_eq!(comments[0].upvote_count, 0);
    assert!(!comments[0].viewer_has_upvoted);
}

#[tokio::test]
async fn creates_and_toggles_comment_upvotes() {
    let store = test_store().await;
    let author = store
        .upsert_user(211, "author", "https://example.com/author.png")
        .await
        .expect("author");
    let voter = store
        .upsert_user(212, "voter", "https://example.com/voter.png")
        .await
        .expect("voter");
    let idea = store
        .create_idea(
            "A comment upvote idea",
            "This body is long enough for a comment upvote test.",
            "This problem statement is long enough to satisfy validation.",
            author.id,
            false,
        )
        .await
        .expect("idea");
    let comment = store
        .create_comment(idea.id, "This comment can be upvoted.", author.id, false)
        .await
        .expect("comment");
    assert_eq!(comment.upvote_count, 0);
    assert!(!comment.viewer_has_upvoted);

    let voted = store
        .set_comment_upvote(comment.id, voter.id, true, false)
        .await
        .expect("comment upvote");
    assert_eq!(voted.upvote_count, 1);
    assert!(voted.viewer_has_upvoted);

    let voted_again = store
        .set_comment_upvote(comment.id, voter.id, true, false)
        .await
        .expect("idempotent comment upvote");
    assert_eq!(voted_again.upvote_count, 1);

    let unvoted = store
        .set_comment_upvote(comment.id, voter.id, false, false)
        .await
        .expect("comment unvote");
    assert_eq!(unvoted.upvote_count, 0);
    assert!(!unvoted.viewer_has_upvoted);
}

#[tokio::test]
async fn list_comments_promotes_only_the_most_upvoted_comment() {
    let store = test_store().await;
    let author = store
        .upsert_user(221, "author", "https://example.com/author.png")
        .await
        .expect("author");
    let voter = store
        .upsert_user(222, "voter", "https://example.com/voter.png")
        .await
        .expect("voter");
    let idea = store
        .create_idea(
            "A sorted comment idea",
            "This body is long enough for a sorted comment list test.",
            "This problem statement is long enough to satisfy validation.",
            author.id,
            false,
        )
        .await
        .expect("idea");
    let first = store
        .create_comment(idea.id, "First chronological comment.", author.id, false)
        .await
        .expect("first comment");
    let second = store
        .create_comment(idea.id, "Second chronological comment.", author.id, false)
        .await
        .expect("second comment");
    let third = store
        .create_comment(idea.id, "Third chronological comment.", author.id, false)
        .await
        .expect("third comment");
    for (comment, created_at) in [
        (&first, "2026-05-18T10:00:00.000Z"),
        (&second, "2026-05-18T10:01:00.000Z"),
        (&third, "2026-05-18T10:02:00.000Z"),
    ] {
        sqlx::query("UPDATE comments SET created_at = ?, updated_at = ? WHERE id = ?")
            .bind(created_at)
            .bind(created_at)
            .bind(comment.id.to_string())
            .execute(store.pool())
            .await
            .expect("set comment timestamp");
    }
    store
        .set_comment_upvote(third.id, voter.id, true, false)
        .await
        .expect("upvote third comment");

    let comments = store
        .list_comments(idea.id, Some(voter.id), false)
        .await
        .expect("comments");
    assert_eq!(comments[0].id, third.id);
    assert_eq!(comments[1].id, first.id);
    assert_eq!(comments[2].id, second.id);
}

#[tokio::test]
async fn marks_whether_viewer_can_edit_idea() {
    let store = test_store().await;
    let author = store
        .upsert_user(11, "author", "https://example.com/author.png")
        .await
        .expect("author");
    let other = store
        .upsert_user(12, "viewer", "https://example.com/viewer.png")
        .await
        .expect("other user");

    let idea = store
        .create_idea(
            "A useful local idea",
            "This body is long enough to satisfy validation.",
            "This problem statement is long enough to satisfy validation.",
            author.id,
            false,
        )
        .await
        .expect("idea");
    assert!(idea.viewer_can_edit);
    assert!(idea.viewer_can_delete);
    assert!(!idea.viewer_can_close);

    let author_view = store
        .idea(idea.id, Some(author.id), false)
        .await
        .expect("author view");
    assert!(author_view.viewer_can_edit);
    assert!(author_view.viewer_can_delete);
    assert!(!author_view.viewer_can_close);

    let other_view = store
        .idea(idea.id, Some(other.id), false)
        .await
        .expect("other view");
    assert!(!other_view.viewer_can_edit);
    assert!(!other_view.viewer_can_delete);
    assert!(!other_view.viewer_can_close);

    let anonymous_view = store
        .idea(idea.id, None, false)
        .await
        .expect("anonymous view");
    assert!(!anonymous_view.viewer_can_edit);
    assert!(!anonymous_view.viewer_can_delete);
    assert!(!anonymous_view.viewer_can_close);
}

#[tokio::test]
async fn dev_seed_is_repeatable() {
    let store = test_store().await;

    let first = store.seed_dev_examples().await.expect("first seed");
    let second = store.seed_dev_examples().await.expect("second seed");

    assert_eq!(first, second);
    assert_eq!(first.users, 4);
    assert_eq!(first.ideas, 6);
    assert_eq!(first.upvotes, 12);

    let ideas = store.list_ideas(None, false).await.expect("ideas");
    assert_eq!(ideas.len(), 6);
    assert!(ideas
        .iter()
        .any(|idea| idea.title == "Simple backup restore drill scheduler"
            && idea.status == IdeaStatus::Closed));
    assert!(ideas.iter().any(|idea| idea.upvote_count == 3));
}

#[tokio::test]
async fn session_round_trip() {
    let store = test_store().await;
    let user = store
        .upsert_user(10, "carol", "https://example.com/c.png")
        .await
        .expect("user");
    store
        .create_session("sid", "csrf", &user, 3600)
        .await
        .expect("session");
    let session = store
        .session("sid")
        .await
        .expect("load session")
        .expect("exists");
    assert_eq!(session.user.login, "carol");
    assert_eq!(session.csrf_token, "csrf");
}
