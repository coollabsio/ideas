CREATE TABLE comment_upvotes (
  comment_id TEXT NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  PRIMARY KEY (comment_id, user_id)
);
CREATE INDEX idx_comment_upvotes_comment ON comment_upvotes(comment_id);
CREATE INDEX idx_comment_upvotes_user ON comment_upvotes(user_id);
