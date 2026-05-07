import type { APIRoute } from 'astro';
import { deleteSession, getSession } from '~/lib/session';
import { GitHubAuthError, getValidAccessToken, toggleUpvote } from '~/lib/github';

export const prerender = false;

interface Body {
  issueNumber?: number;
  upvoted?: boolean;
}

export const POST: APIRoute = async ({ request, cookies }) => {
  const session = getSession(cookies.get('sid')?.value);
  if (!session) return new Response('Unauthorized', { status: 401 });

  const csrfHeader = request.headers.get('x-csrf-token');
  if (csrfHeader !== session.csrfToken) {
    return new Response('Bad CSRF token', { status: 403 });
  }

  let body: Body;
  try {
    body = (await request.json()) as Body;
  } catch {
    return new Response('Invalid JSON', { status: 400 });
  }
  if (typeof body.issueNumber !== 'number' || !Number.isInteger(body.issueNumber) || typeof body.upvoted !== 'boolean') {
    return new Response('Missing fields', { status: 400 });
  }

  try {
    const token = await getValidAccessToken(session);
    const result = await toggleUpvote(body.issueNumber, body.upvoted, token, session.login);
    return Response.json(result);
  } catch (err) {
    if (err instanceof GitHubAuthError) {
      deleteSession(session.sid);
      cookies.delete('sid', { path: '/' });
      return new Response('Session expired', { status: 401 });
    }
    return new Response((err as Error).message, { status: 502 });
  }
};
