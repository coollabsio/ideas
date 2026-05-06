import type { APIRoute } from 'astro';
import { config } from '~/lib/config';
import { listIdeas } from '~/lib/github';
import { getSession } from '~/lib/session';

export const prerender = false;

const CACHE_TTL_MS = 30_000;
let anonCache: { at: number; data: unknown } | null = null;

export const GET: APIRoute = async ({ cookies }) => {
  const session = getSession(cookies.get('sid')?.value);
  const token = session?.accessToken ?? config.githubToken;

  if (!session && anonCache && Date.now() - anonCache.at < CACHE_TTL_MS) {
    return Response.json(anonCache.data);
  }

  try {
    const ideas = await listIdeas(token);
    if (!session) anonCache = { at: Date.now(), data: ideas };
    return Response.json(ideas);
  } catch (err) {
    return new Response((err as Error).message, { status: 502 });
  }
};
