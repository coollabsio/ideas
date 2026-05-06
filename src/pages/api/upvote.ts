import type { APIRoute } from 'astro';
import { getSession } from '~/lib/session';
import { toggleUpvote } from '~/lib/github';

export const prerender = false;

interface Body {
  discussionId?: string;
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
  if (!body.discussionId || typeof body.upvoted !== 'boolean') {
    return new Response('Missing fields', { status: 400 });
  }

  try {
    const result = await toggleUpvote(body.discussionId, body.upvoted, session.accessToken);
    return Response.json(result);
  } catch (err) {
    return new Response((err as Error).message, { status: 502 });
  }
};
