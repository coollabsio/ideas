import { config } from './config';
import { listIdeas, type Idea } from './github';

export const ANON_IDEAS_CACHE_TTL_MS = 60_000;

let cachedIdeas: { at: number; data: Idea[] } | null = null;
let pendingIdeas: Promise<Idea[]> | null = null;
let cacheVersion = 0;

function cacheFresh(): boolean {
  return Boolean(cachedIdeas && Date.now() - cachedIdeas.at < ANON_IDEAS_CACHE_TTL_MS);
}

export async function getAnonIdeas(): Promise<Idea[]> {
  if (cacheFresh() && cachedIdeas) return cachedIdeas.data;

  const version = cacheVersion;
  pendingIdeas ??= listIdeas(config.githubToken).then((ideas) => {
    if (version === cacheVersion) cachedIdeas = { at: Date.now(), data: ideas };
    if (version === cacheVersion) pendingIdeas = null;
    return ideas;
  }, (err) => {
    if (version === cacheVersion) pendingIdeas = null;
    throw err;
  });

  return pendingIdeas;
}

export function invalidateIdeasCache(): void {
  cacheVersion += 1;
  cachedIdeas = null;
  pendingIdeas = null;
}
