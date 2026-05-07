import type { APIRoute } from 'astro';
import { GitHubAuthError, getValidAccessToken, listIdeas } from '~/lib/github';
import { getAnonIdeas } from '~/lib/ideas-cache';
import { deleteSession, getSession } from '~/lib/session';

export const prerender = false;

export const GET: APIRoute = async ({ cookies }) => {
  const session = getSession(cookies.get('sid')?.value);

  try {
    if (!session) {
      return Response.json(await getAnonIdeas(), {
        headers: {
          'Cache-Control': 'public, max-age=0, s-maxage=60, stale-while-revalidate=30',
          'CDN-Cache-Control': 'max-age=60',
          Vary: 'Cookie',
        },
      });
    }

    const token = await getValidAccessToken(session);
    const ideas = await listIdeas(token, session.login);
    return Response.json(ideas, {
      headers: { 'Cache-Control': 'private, no-store', Vary: 'Cookie' },
    });
  } catch (err) {
    if (err instanceof GitHubAuthError && session) {
      deleteSession(session.sid);
      cookies.delete('sid', { path: '/' });
      return new Response('Session expired', { status: 401 });
    }
    return new Response((err as Error).message, { status: 502 });
  }
};
