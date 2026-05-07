import type { APIRoute } from 'astro';
import { deleteSession, getSession } from '~/lib/session';
import { GitHubAuthError, createIssue, getValidAccessToken } from '~/lib/github';

export const prerender = false;

interface Body {
  title?: string;
  body?: string;
}

const TITLE_MIN = 10;
const TITLE_MAX = 300;
const BODY_MIN = 30;
const BODY_MAX = 10000;

export const POST: APIRoute = async ({ request, cookies }) => {
  const session = getSession(cookies.get('sid')?.value);
  if (!session) return new Response('Unauthorized', { status: 401 });

  const csrfHeader = request.headers.get('x-csrf-token');
  if (csrfHeader !== session.csrfToken) {
    return new Response('Bad CSRF token', { status: 403 });
  }

  let payload: Body;
  try {
    payload = (await request.json()) as Body;
  } catch {
    return new Response('Invalid JSON', { status: 400 });
  }

  const title = (payload.title ?? '').trim();
  const body = (payload.body ?? '').trim();

  if (title.length < TITLE_MIN || title.length > TITLE_MAX) {
    return new Response(
      `Title must be ${TITLE_MIN}–${TITLE_MAX} characters`,
      { status: 400 }
    );
  }
  if (body.length < BODY_MIN || body.length > BODY_MAX) {
    return new Response(
      `Body must be ${BODY_MIN}–${BODY_MAX} characters`,
      { status: 400 }
    );
  }

  try {
    const token = await getValidAccessToken(session);
    const idea = await createIssue(title, body, token);
    return Response.json(idea);
  } catch (err) {
    if (err instanceof GitHubAuthError) {
      deleteSession(session.sid);
      cookies.delete('sid', { path: '/' });
      return new Response('Session expired', { status: 401 });
    }
    return new Response((err as Error).message, { status: 502 });
  }
};
