import { config } from './config';
import {
  updateSessionTokens,
  type OAuthTokenSet,
  type Session,
} from './session';

const GRAPHQL_URL = 'https://api.github.com/graphql';
const REST_URL = 'https://api.github.com';
const TOKEN_URL = 'https://github.com/login/oauth/access_token';
const UA = 'coollabs-ideas-app';
const REFRESH_LEAD_SEC = 60;
const IDEA_LABEL = 'idea';
const LEGACY_UPVOTE_RE = /Legacy Discussion upvotes:\s*(\d+)/i;

export interface Idea {
  id: string;
  number: number;
  title: string;
  bodyText: string;
  url: string;
  upvoteCount: number;
  viewerHasUpvoted: boolean;
  author: { login: string; avatarUrl: string } | null;
  category: { name: string };
  createdAt: string;
  closed: boolean;
  legacyUpvoteCount: number;
}

export class GitHubAuthError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'GitHubAuthError';
  }
}

interface GqlResponse<T> {
  data?: T;
  errors?: Array<{ message: string; type?: string }>;
}

interface TokenResponse {
  access_token?: string;
  refresh_token?: string;
  expires_in?: number;
  refresh_token_expires_in?: number;
  token_type?: string;
  error?: string;
  error_description?: string;
}

interface GitHubUser {
  login: string;
  avatar_url: string;
}

interface GitHubIssue {
  id: number;
  node_id: string;
  number: number;
  title: string;
  body: string | null;
  html_url: string;
  state: 'open' | 'closed';
  created_at: string;
  user: GitHubUser | null;
  reactions?: { '+1'?: number };
  pull_request?: unknown;
}

interface GitHubReaction {
  id: number;
  content: string;
  user: GitHubUser | null;
}

export async function exchangeCodeForToken(code: string): Promise<OAuthTokenSet> {
  const res = await fetch(TOKEN_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({
      client_id: config.githubClientId,
      client_secret: config.githubClientSecret,
      code,
      redirect_uri: `${config.baseUrl}/api/auth/callback`,
    }),
  });
  if (!res.ok) throw new Error(`Token exchange HTTP ${res.status}: ${await res.text()}`);
  const data = (await res.json()) as TokenResponse;
  if (!data.access_token) {
    throw new Error(`OAuth error: ${data.error ?? 'no token'} ${data.error_description ?? ''}`);
  }
  return {
    accessToken: data.access_token,
    refreshToken: data.refresh_token ?? null,
    expiresIn: data.expires_in ?? null,
    refreshTokenExpiresIn: data.refresh_token_expires_in ?? null,
  };
}

async function refreshAccessToken(refreshToken: string): Promise<OAuthTokenSet> {
  const res = await fetch(TOKEN_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({
      client_id: config.githubClientId,
      client_secret: config.githubClientSecret,
      grant_type: 'refresh_token',
      refresh_token: refreshToken,
    }),
  });
  if (!res.ok) throw new GitHubAuthError(`Refresh HTTP ${res.status}: ${await res.text()}`);
  const data = (await res.json()) as TokenResponse;
  if (!data.access_token) {
    throw new GitHubAuthError(`Refresh failed: ${data.error ?? 'no token'} ${data.error_description ?? ''}`);
  }
  return {
    accessToken: data.access_token,
    refreshToken: data.refresh_token ?? null,
    expiresIn: data.expires_in ?? null,
    refreshTokenExpiresIn: data.refresh_token_expires_in ?? null,
  };
}

export async function getValidAccessToken(session: Session): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  const expires = session.tokenExpiresAt;
  if (!expires || expires - now > REFRESH_LEAD_SEC) {
    return session.accessToken;
  }
  if (!session.refreshToken) {
    throw new GitHubAuthError('Token expired and no refresh token available');
  }
  if (session.refreshTokenExpiresAt && session.refreshTokenExpiresAt < now) {
    throw new GitHubAuthError('Refresh token expired');
  }
  const tokens = await refreshAccessToken(session.refreshToken);
  updateSessionTokens(session.sid, tokens);
  session.accessToken = tokens.accessToken;
  session.refreshToken = tokens.refreshToken ?? session.refreshToken;
  session.tokenExpiresAt = tokens.expiresIn ? now + tokens.expiresIn : null;
  session.refreshTokenExpiresAt = tokens.refreshTokenExpiresIn
    ? now + tokens.refreshTokenExpiresIn
    : session.refreshTokenExpiresAt;
  return tokens.accessToken;
}

async function gql<T>(
  query: string,
  variables: Record<string, unknown>,
  token: string
): Promise<T> {
  const res = await fetch(GRAPHQL_URL, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
      'User-Agent': UA,
    },
    body: JSON.stringify({ query, variables }),
  });
  if (res.status === 401) {
    throw new GitHubAuthError(`GitHub GraphQL 401: ${await res.text()}`);
  }
  if (!res.ok) {
    throw new Error(`GitHub GraphQL HTTP ${res.status}: ${await res.text()}`);
  }
  const json = (await res.json()) as GqlResponse<T>;
  if (json.errors?.length) {
    const messages = json.errors.map((e) => e.message).join('; ');
    throw new Error(`GitHub GraphQL errors: ${messages}`);
  }
  if (!json.data) throw new Error('GitHub GraphQL returned no data');
  return json.data;
}

function repoPath(): string {
  return `/repos/${config.repo.owner}/${config.repo.name}`;
}

async function gh<T>(
  path: string,
  token: string,
  init: RequestInit = {}
): Promise<T> {
  const res = await fetch(`${REST_URL}${path}`, {
    ...init,
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: 'application/vnd.github+json',
      'Content-Type': 'application/json',
      'User-Agent': UA,
      'X-GitHub-Api-Version': '2022-11-28',
      ...init.headers,
    },
  });
  if (res.status === 401) {
    throw new GitHubAuthError(`GitHub REST 401: ${await res.text()}`);
  }
  if (res.status === 204) {
    return undefined as T;
  }
  if (!res.ok) {
    throw new Error(`GitHub REST HTTP ${res.status}: ${await res.text()}`);
  }
  return (await res.json()) as T;
}

async function ghPages<T>(path: string, token: string): Promise<T[]> {
  const joiner = path.includes('?') ? '&' : '?';
  const out: T[] = [];
  for (let page = 1; ; page += 1) {
    const batch = await gh<T[]>(`${path}${joiner}per_page=100&page=${page}`, token);
    out.push(...batch);
    if (batch.length < 100) return out;
  }
}

function legacyUpvotes(body: string | null): number {
  const match = body?.match(LEGACY_UPVOTE_RE);
  return match ? Number.parseInt(match[1] ?? '0', 10) : 0;
}

function displayBody(body: string | null): string {
  return (body ?? '').replace(/\n---\nMigrated from:[\s\S]*$/i, '').trim();
}

function toIdea(
  issue: GitHubIssue,
  viewerHasUpvoted = false
): Idea {
  const legacyUpvoteCount = legacyUpvotes(issue.body);
  const reactionUpvotes = issue.reactions?.['+1'] ?? 0;
  return {
    id: issue.node_id,
    number: issue.number,
    title: issue.title,
    bodyText: displayBody(issue.body),
    url: issue.html_url,
    upvoteCount: legacyUpvoteCount + reactionUpvotes,
    viewerHasUpvoted,
    author: issue.user
      ? { login: issue.user.login, avatarUrl: issue.user.avatar_url }
      : null,
    category: { name: config.ideasCategory },
    createdAt: issue.created_at,
    closed: issue.state === 'closed',
    legacyUpvoteCount,
  };
}

async function issueHasViewerUpvote(
  issueNumber: number,
  viewerLogin: string,
  token: string
): Promise<boolean> {
  const reactions = await listIssueUpvoteReactions(issueNumber, token);
  const login = viewerLogin.toLowerCase();
  return reactions.some((r) => r.user?.login.toLowerCase() === login);
}

async function listIssueUpvoteReactions(
  issueNumber: number,
  token: string
): Promise<GitHubReaction[]> {
  return ghPages<GitHubReaction>(
    `${repoPath()}/issues/${issueNumber}/reactions?content=%2B1`,
    token
  );
}

export async function listIdeas(token: string, viewerLogin?: string): Promise<Idea[]> {
  const issues = await ghPages<GitHubIssue>(
    `${repoPath()}/issues?state=all&labels=${encodeURIComponent(IDEA_LABEL)}`,
    token
  );
  const ideas = await Promise.all(
    issues.filter((issue) => !issue.pull_request).map(async (issue) => {
      const viewerHasUpvoted = viewerLogin
        ? await issueHasViewerUpvote(issue.number, viewerLogin, token)
        : false;
      return toIdea(issue, viewerHasUpvoted);
    })
  );
  return ideas.sort((a, b) => b.upvoteCount - a.upvoteCount);
}

export async function toggleUpvote(
  issueNumber: number,
  currentlyUpvoted: boolean,
  token: string,
  viewerLogin: string
): Promise<{ upvoteCount: number; viewerHasUpvoted: boolean }> {
  if (currentlyUpvoted) {
    const reaction = (await listIssueUpvoteReactions(issueNumber, token)).find(
      (r) => r.user?.login.toLowerCase() === viewerLogin.toLowerCase()
    );
    if (reaction) {
      await gh<void>(
        `${repoPath()}/issues/${issueNumber}/reactions/${reaction.id}`,
        token,
        { method: 'DELETE' }
      );
    }
  } else {
    try {
      await gh<GitHubReaction>(
        `${repoPath()}/issues/${issueNumber}/reactions`,
        token,
        { method: 'POST', body: JSON.stringify({ content: '+1' }) }
      );
    } catch (err) {
      // GitHub returns 422 if this user already has this exact reaction.
      if (!(err as Error).message.includes('HTTP 422')) throw err;
    }
  }

  const issue = await gh<GitHubIssue>(`${repoPath()}/issues/${issueNumber}`, token);
  const viewerHasUpvoted = await issueHasViewerUpvote(issueNumber, viewerLogin, token);
  return {
    upvoteCount: toIdea(issue).upvoteCount,
    viewerHasUpvoted,
  };
}

async function ensureIdeaLabel(issueNumber: number): Promise<void> {
  try {
    await gh<unknown>(`${repoPath()}/issues/${issueNumber}/labels`, config.githubToken, {
      method: 'POST',
      body: JSON.stringify({ labels: [IDEA_LABEL] }),
    });
  } catch (err) {
    throw new Error(
      `Failed to add ${IDEA_LABEL} label to issue #${issueNumber}: ${(err as Error).message}`
    );
  }
}

export async function createIssue(
  title: string,
  body: string,
  token: string
): Promise<Idea> {
  const issue = await gh<GitHubIssue>(repoPath() + '/issues', token, {
    method: 'POST',
    body: JSON.stringify({ title, body }),
  });
  await ensureIdeaLabel(issue.number);
  return toIdea(issue, false);
}

export async function fetchViewer(
  token: string
): Promise<{ login: string; avatarUrl: string }> {
  const user = await gh<GitHubUser>('/user', token);
  return { login: user.login, avatarUrl: user.avatar_url };
}

export async function listDiscussionsForMigration(token: string): Promise<Idea[]> {
  const data = await gql<{ repository: { discussions: { nodes: Idea[] } } }>(
    `query($owner: String!, $name: String!) {
      repository(owner: $owner, name: $name) {
        discussions(first: 100, orderBy: {field: UPDATED_AT, direction: DESC}) {
          nodes {
            id number title bodyText url upvoteCount viewerHasUpvoted closed
            author { login avatarUrl }
            category { name }
            createdAt
          }
        }
      }
    }`,
    { owner: config.repo.owner, name: config.repo.name },
    token
  );
  return data.repository.discussions.nodes
    .filter((n) => n.category.name === config.ideasCategory)
    .map((n) => ({ ...n, legacyUpvoteCount: n.upvoteCount }))
    .sort((a, b) => b.upvoteCount - a.upvoteCount);
}
