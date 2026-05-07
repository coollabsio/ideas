import type { APIRoute } from 'astro';
import { config } from '~/lib/config';
import { GitHubAuthError, getValidAccessToken, listIdeas } from '~/lib/github';
import { deleteSession, getSession } from '~/lib/session';

export const prerender = false;

const CACHE_TTL_MS = 30_000;
let anonCache: { at: number; data: unknown } | null = null;

export const GET: APIRoute = async ({ cookies }) => {
  const session = getSession(cookies.get('sid')?.value);

  if (!session && anonCache && Date.now() - anonCache.at < CACHE_TTL_MS) {
    return Response.json(anonCache.data);
  }

  try {
    const token = session ? await getValidAccessToken(session) : config.githubToken;
    const ideas = await listIdeas(token, session?.login);
    if (!session) anonCache = { at: Date.now(), data: ideas };
    return Response.json(ideas);
  } catch (err) {
    if (err instanceof GitHubAuthError && session) {
      deleteSession(session.sid);
      cookies.delete('sid', { path: '/' });
      return new Response('Session expired', { status: 401 });
    }
    return new Response((err as Error).message, { status: 502 });
  }
};
