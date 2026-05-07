import { config } from './config';
import {
  updateSessionTokens,
  type OAuthTokenSet,
  type Session,
} from './session';

const GRAPHQL_URL = 'https://api.github.com/graphql';
const TOKEN_URL = 'https://github.com/login/oauth/access_token';
const UA = 'coollabs-ideas-app';
const REFRESH_LEAD_SEC = 60;

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

let cachedCategoryId: string | null = null;
let cachedRepoId: string | null = null;

export async function getRepositoryId(token: string): Promise<string> {
  if (cachedRepoId) return cachedRepoId;
  const data = await gql<{ repository: { id: string } }>(
    `query($owner: String!, $name: String!) {
      repository(owner: $owner, name: $name) { id }
    }`,
    { owner: config.repo.owner, name: config.repo.name },
    token
  );
  cachedRepoId = data.repository.id;
  return cachedRepoId;
}

export async function getIdeasCategoryId(token: string): Promise<string> {
  if (cachedCategoryId) return cachedCategoryId;
  const data = await gql<{
    repository: { discussionCategories: { nodes: Array<{ id: string; name: string }> } };
  }>(
    `query($owner: String!, $name: String!) {
      repository(owner: $owner, name: $name) {
        discussionCategories(first: 25) { nodes { id name } }
      }
    }`,
    { owner: config.repo.owner, name: config.repo.name },
    token
  );
  const cat = data.repository.discussionCategories.nodes.find(
    (c) => c.name === config.ideasCategory
  );
  if (!cat) {
    throw new Error(
      `Discussion category "${config.ideasCategory}" not found in ${config.repo.owner}/${config.repo.name}`
    );
  }
  cachedCategoryId = cat.id;
  return cat.id;
}

export async function listIdeas(token: string): Promise<Idea[]> {
  const categoryId = await getIdeasCategoryId(token);
  const data = await gql<{ repository: { discussions: { nodes: Idea[] } } }>(
    `query($owner: String!, $name: String!, $cat: ID!) {
      repository(owner: $owner, name: $name) {
        discussions(first: 100, categoryId: $cat, orderBy: {field: UPDATED_AT, direction: DESC}) {
          nodes {
            id number title bodyText url upvoteCount viewerHasUpvoted
            author { login avatarUrl }
            category { name }
            createdAt
          }
        }
      }
    }`,
    { owner: config.repo.owner, name: config.repo.name, cat: categoryId },
    token
  );
  return data.repository.discussions.nodes
    .filter((n) => n.category.name === config.ideasCategory)
    .sort((a, b) => b.upvoteCount - a.upvoteCount);
}

export async function toggleUpvote(
  discussionId: string,
  currentlyUpvoted: boolean,
  token: string
): Promise<{ upvoteCount: number; viewerHasUpvoted: boolean }> {
  const op = currentlyUpvoted ? 'removeUpvote' : 'addUpvote';
  const data = await gql<
    Record<string, { subject: { upvoteCount: number; viewerHasUpvoted: boolean } }>
  >(
    `mutation($id: ID!) {
      ${op}(input: { subjectId: $id }) {
        subject { ... on Discussion { upvoteCount viewerHasUpvoted } }
      }
    }`,
    { id: discussionId },
    token
  );
  return data[op].subject;
}

export async function createDiscussion(
  title: string,
  body: string,
  token: string
): Promise<Idea> {
  const [repositoryId, categoryId] = await Promise.all([
    getRepositoryId(token),
    getIdeasCategoryId(token),
  ]);
  const data = await gql<{
    createDiscussion: { discussion: Omit<Idea, 'category'> };
  }>(
    `mutation($repo: ID!, $cat: ID!, $title: String!, $body: String!) {
      createDiscussion(input: {repositoryId: $repo, categoryId: $cat, title: $title, body: $body}) {
        discussion {
          id number title bodyText url upvoteCount viewerHasUpvoted
          author { login avatarUrl }
          createdAt
        }
      }
    }`,
    { repo: repositoryId, cat: categoryId, title, body },
    token
  );
  return {
    ...data.createDiscussion.discussion,
    category: { name: config.ideasCategory },
  };
}

export async function fetchViewer(
  token: string
): Promise<{ login: string; avatarUrl: string }> {
  const data = await gql<{ viewer: { login: string; avatarUrl: string } }>(
    `query { viewer { login avatarUrl } }`,
    {},
    token
  );
  return data.viewer;
}
