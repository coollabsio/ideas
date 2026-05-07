import 'dotenv/config';
import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const GRAPHQL_URL = 'https://api.github.com/graphql';
const REST_URL = 'https://api.github.com';
const UA = 'coollabs-ideas-migration';
const OWNER = 'coollabsio';
const REPO = 'ideas';
const CATEGORY = 'Ideas';
const MAP_PATH = resolve('data/discussion-issue-map.json');

interface Discussion {
  id: string;
  number: number;
  title: string;
  bodyText: string;
  url: string;
  upvoteCount: number;
  closed: boolean;
  category: { name: string };
}

interface Issue {
  number: number;
  html_url: string;
  body: string | null;
}

interface MigrationMapEntry {
  discussionId: string;
  discussionNumber: number;
  discussionUrl: string;
  issueNumber: number;
  issueUrl: string;
  legacyUpvoteCount: number;
}

type MigrationMap = Record<string, MigrationMapEntry>;

const token = process.env.GITHUB_TOKEN;
if (!token) throw new Error('Missing GITHUB_TOKEN');

const apply = process.argv.includes('--apply');

async function gql<T>(query: string, variables: Record<string, unknown>): Promise<T> {
  const res = await fetch(GRAPHQL_URL, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
      'User-Agent': UA,
    },
    body: JSON.stringify({ query, variables }),
  });
  if (!res.ok) throw new Error(`GraphQL HTTP ${res.status}: ${await res.text()}`);
  const json = (await res.json()) as { data?: T; errors?: Array<{ message: string }> };
  if (json.errors?.length) throw new Error(json.errors.map((e) => e.message).join('; '));
  if (!json.data) throw new Error('GraphQL returned no data');
  return json.data;
}

async function gh<T>(path: string, init: RequestInit = {}): Promise<T> {
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
  if (!res.ok) throw new Error(`REST HTTP ${res.status}: ${await res.text()}`);
  return (await res.json()) as T;
}

async function ghPages<T>(path: string): Promise<T[]> {
  const joiner = path.includes('?') ? '&' : '?';
  const out: T[] = [];
  for (let page = 1; ; page += 1) {
    const batch = await gh<T[]>(`${path}${joiner}per_page=100&page=${page}`);
    out.push(...batch);
    if (batch.length < 100) return out;
  }
}

async function listDiscussions(): Promise<Discussion[]> {
  const data = await gql<{ repository: { discussions: { nodes: Discussion[] } } }>(
    `query($owner: String!, $name: String!) {
      repository(owner: $owner, name: $name) {
        discussions(first: 100, orderBy: {field: UPDATED_AT, direction: DESC}) {
          nodes {
            id number title bodyText url upvoteCount closed
            category { name }
          }
        }
      }
    }`,
    { owner: OWNER, name: REPO }
  );
  return data.repository.discussions.nodes.filter((d) => d.category.name === CATEGORY);
}

async function listMigratedIssues(): Promise<Issue[]> {
  return ghPages<Issue>(
    `/repos/${OWNER}/${REPO}/issues?state=all&labels=idea,migrated-from-discussion`
  );
}

async function createIssue(discussion: Discussion): Promise<Issue> {
  const body = `${discussion.bodyText}

---
Migrated from: ${discussion.url}
Legacy Discussion upvotes: ${discussion.upvoteCount}
`;

  return gh<Issue>(`/repos/${OWNER}/${REPO}/issues`, {
    method: 'POST',
    body: JSON.stringify({
      title: discussion.title,
      body,
      labels: ['idea', 'migrated-from-discussion'],
    }),
  });
}

async function loadMap(): Promise<MigrationMap> {
  const file = Bun.file(MAP_PATH);
  if (!(await file.exists())) return {};
  return (await file.json()) as MigrationMap;
}

async function saveMap(map: MigrationMap): Promise<void> {
  mkdirSync(dirname(MAP_PATH), { recursive: true });
  await Bun.write(MAP_PATH, JSON.stringify(map, null, 2) + '\n');
}

const map = await loadMap();
const [discussions, existingIssues] = await Promise.all([
  listDiscussions(),
  listMigratedIssues(),
]);

console.log(`${apply ? 'APPLY' : 'DRY RUN'}: ${discussions.length} discussions found`);

for (const discussion of discussions) {
  const key = String(discussion.number);
  if (map[key]) {
    console.log(`skip #${discussion.number}: already mapped to issue #${map[key].issueNumber}`);
    continue;
  }

  const existing = existingIssues.find((issue) => issue.body?.includes(`Migrated from: ${discussion.url}`));
  if (existing) {
    map[key] = {
      discussionId: discussion.id,
      discussionNumber: discussion.number,
      discussionUrl: discussion.url,
      issueNumber: existing.number,
      issueUrl: existing.html_url,
      legacyUpvoteCount: discussion.upvoteCount,
    };
    console.log(`map #${discussion.number}: existing issue #${existing.number}`);
    continue;
  }

  if (!apply) {
    console.log(`would create issue for discussion #${discussion.number}: ${discussion.title}`);
    continue;
  }

  const issue = await createIssue(discussion);
  map[key] = {
    discussionId: discussion.id,
    discussionNumber: discussion.number,
    discussionUrl: discussion.url,
    issueNumber: issue.number,
    issueUrl: issue.html_url,
    legacyUpvoteCount: discussion.upvoteCount,
  };
  console.log(`created issue #${issue.number} from discussion #${discussion.number}`);
}

if (apply) {
  await saveMap(map);
  console.log(`mapping written to ${MAP_PATH}`);
} else {
  console.log(`dry run complete; no mapping written`);
}
