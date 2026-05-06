import { randomBytes } from 'node:crypto';
import { db } from './db';

export interface Session {
  sid: string;
  accessToken: string;
  refreshToken: string | null;
  tokenExpiresAt: number | null;
  refreshTokenExpiresAt: number | null;
  login: string;
  avatarUrl: string;
  csrfToken: string;
  expiresAt: number;
}

interface SessionRow {
  sid: string;
  accessToken: string;
  refreshToken: string | null;
  tokenExpiresAt: number | null;
  refreshTokenExpiresAt: number | null;
  login: string;
  avatarUrl: string;
  csrfToken: string;
  expiresAt: number;
}

export interface OAuthTokenSet {
  accessToken: string;
  refreshToken?: string | null;
  expiresIn?: number | null;
  refreshTokenExpiresIn?: number | null;
}

const insertSession = db.prepare(`
  INSERT INTO sessions (
    sid, access_token, refresh_token, token_expires_at, refresh_token_expires_at,
    login, avatar_url, csrf_token, created_at, expires_at
  ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
`);

const updateTokens = db.prepare(`
  UPDATE sessions
  SET access_token = ?, refresh_token = ?, token_expires_at = ?, refresh_token_expires_at = ?
  WHERE sid = ?
`);

const selectSession = db.prepare(`
  SELECT sid,
         access_token AS accessToken,
         refresh_token AS refreshToken,
         token_expires_at AS tokenExpiresAt,
         refresh_token_expires_at AS refreshTokenExpiresAt,
         login,
         avatar_url AS avatarUrl,
         csrf_token AS csrfToken,
         expires_at AS expiresAt
  FROM sessions
  WHERE sid = ? AND expires_at > ?
`);

const deleteSessionStmt = db.prepare('DELETE FROM sessions WHERE sid = ?');

const insertState = db.prepare('INSERT INTO oauth_state (state, created_at) VALUES (?, ?)');
const selectState = db.prepare('SELECT created_at FROM oauth_state WHERE state = ?');
const deleteState = db.prepare('DELETE FROM oauth_state WHERE state = ?');

export function createSession(
  tokens: OAuthTokenSet,
  login: string,
  avatarUrl: string,
  ttlSec: number
): Session {
  const sid = randomBytes(32).toString('hex');
  const csrfToken = randomBytes(32).toString('hex');
  const now = Math.floor(Date.now() / 1000);
  const expiresAt = now + ttlSec;
  const tokenExpiresAt = tokens.expiresIn ? now + tokens.expiresIn : null;
  const refreshTokenExpiresAt = tokens.refreshTokenExpiresIn
    ? now + tokens.refreshTokenExpiresIn
    : null;
  insertSession.run(
    sid,
    tokens.accessToken,
    tokens.refreshToken ?? null,
    tokenExpiresAt,
    refreshTokenExpiresAt,
    login,
    avatarUrl,
    csrfToken,
    now,
    expiresAt
  );
  return {
    sid,
    accessToken: tokens.accessToken,
    refreshToken: tokens.refreshToken ?? null,
    tokenExpiresAt,
    refreshTokenExpiresAt,
    login,
    avatarUrl,
    csrfToken,
    expiresAt,
  };
}

export function updateSessionTokens(sid: string, tokens: OAuthTokenSet): void {
  const now = Math.floor(Date.now() / 1000);
  const tokenExpiresAt = tokens.expiresIn ? now + tokens.expiresIn : null;
  const refreshTokenExpiresAt = tokens.refreshTokenExpiresIn
    ? now + tokens.refreshTokenExpiresIn
    : null;
  updateTokens.run(
    tokens.accessToken,
    tokens.refreshToken ?? null,
    tokenExpiresAt,
    refreshTokenExpiresAt,
    sid
  );
}

export function getSession(sid: string | undefined): Session | null {
  if (!sid) return null;
  const now = Math.floor(Date.now() / 1000);
  const row = selectSession.get(sid, now) as SessionRow | undefined;
  return row ?? null;
}

export function deleteSession(sid: string): void {
  deleteSessionStmt.run(sid);
}

export function createOAuthState(): string {
  const state = randomBytes(32).toString('hex');
  insertState.run(state, Math.floor(Date.now() / 1000));
  return state;
}

export function consumeOAuthState(state: string): boolean {
  const row = selectState.get(state) as { created_at: number } | undefined;
  if (!row) return false;
  deleteState.run(state);
  const now = Math.floor(Date.now() / 1000);
  return row.created_at >= now - 600;
}
