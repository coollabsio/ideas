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
        )
        .await
        .expect("idea");
    assert_eq!(idea.upvote_count, 0);

    let voted = store
        .set_upvote(idea.id, other.id, true)
        .await
        .expect("vote");
    assert_eq!(voted.upvote_count, 1);
    assert!(voted.viewer_has_upvoted);

    let voted_again = store
        .set_upvote(idea.id, other.id, true)
        .await
        .expect("idempotent vote");
    assert_eq!(voted_again.upvote_count, 1);

    let unvoted = store
        .set_upvote(idea.id, other.id, false)
        .await
        .expect("unvote");
    assert_eq!(unvoted.upvote_count, 0);
    assert!(!unvoted.viewer_has_upvoted);
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
