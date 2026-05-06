import { config } from './config';

const GRAPHQL_URL = 'https://api.github.com/graphql';
const UA = 'coollabs-ideas-app';

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

interface GqlResponse<T> {
  data?: T;
  errors?: Array<{ message: string }>;
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
  if (!res.ok) {
    throw new Error(`GitHub GraphQL HTTP ${res.status}: ${await res.text()}`);
  }
  const json = (await res.json()) as GqlResponse<T>;
  if (json.errors?.length) {
    throw new Error(`GitHub GraphQL errors: ${json.errors.map((e) => e.message).join('; ')}`);
  }
  if (!json.data) throw new Error('GitHub GraphQL returned no data');
  return json.data;
}

let cachedCategoryId: string | null = null;

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
            id
            number
            title
            bodyText
            url
            upvoteCount
            viewerHasUpvoted
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
  const data = await gql<Record<string, { subject: { upvoteCount: number; viewerHasUpvoted: boolean } }>>(
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

export async function fetchUser(
  token: string
): Promise<{ login: string; avatar_url: string }> {
  const res = await fetch('https://api.github.com/user', {
    headers: { Authorization: `Bearer ${token}`, 'User-Agent': UA },
  });
  if (!res.ok) throw new Error(`GitHub /user HTTP ${res.status}: ${await res.text()}`);
  return (await res.json()) as { login: string; avatar_url: string };
}
