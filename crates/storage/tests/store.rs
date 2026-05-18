use ideas_storage::Store;
use uuid::Uuid;

async fn test_store() -> Store {
    let path = std::env::temp_dir().join(format!("ideas-test-{}.db", Uuid::new_v4()));
    let store = Store::connect(path.to_str().expect("utf8 temp path"))
        .await
        .expect("connect");
    store.migrate().await.expect("migrate");
    store
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
        .any(|idea| idea.title == "Simple backup restore drill scheduler" && idea.closed));
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
