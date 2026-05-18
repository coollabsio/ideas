-- no-transaction
PRAGMA foreign_keys = OFF;

CREATE TEMP TABLE upvotes_backup AS
SELECT idea_id, user_id, created_at
FROM upvotes;

CREATE TABLE ideas_new (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL CHECK(length(title) BETWEEN 10 AND 300),
  body TEXT NOT NULL CHECK(length(body) BETWEEN 30 AND 10000),
  author_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open', 'closed')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

INSERT INTO ideas_new (id, title, body, author_user_id, status, created_at, updated_at)
SELECT id, title, body, author_user_id, CASE WHEN status = 'inprogress' THEN 'open' ELSE status END, created_at, updated_at
FROM ideas;

DROP TABLE ideas;
ALTER TABLE ideas_new RENAME TO ideas;

CREATE INDEX idx_ideas_status_created ON ideas(status, created_at DESC);
CREATE INDEX idx_ideas_author ON ideas(author_user_id);

INSERT OR IGNORE INTO upvotes (idea_id, user_id, created_at)
SELECT b.idea_id, b.user_id, b.created_at
FROM upvotes_backup b
JOIN ideas i ON i.id = b.idea_id
JOIN users u ON u.id = b.user_id;

DROP TABLE upvotes_backup;

PRAGMA foreign_keys = ON;
