import Database from 'better-sqlite3';
import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const DB_PATH = resolve(process.env.DB_PATH ?? './data/sessions.db');

mkdirSync(dirname(DB_PATH), { recursive: true });

export const db = new Database(DB_PATH);
db.pragma('journal_mode = WAL');
db.pragma('foreign_keys = ON');

db.exec(`
  CREATE TABLE IF NOT EXISTS sessions (
    sid TEXT PRIMARY KEY,
    access_token TEXT NOT NULL,
    login TEXT NOT NULL,
    avatar_url TEXT NOT NULL,
    csrf_token TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
  );
  CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);

  CREATE TABLE IF NOT EXISTS oauth_state (
    state TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL
  );
  CREATE INDEX IF NOT EXISTS idx_oauth_state_created ON oauth_state(created_at);
`);

export function sweep(): void {
  const now = Math.floor(Date.now() / 1000);
  db.prepare('DELETE FROM sessions WHERE expires_at < ?').run(now);
  db.prepare('DELETE FROM oauth_state WHERE created_at < ?').run(now - 600);
}

sweep();
const interval = setInterval(sweep, 3600 * 1000);
interval.unref();
