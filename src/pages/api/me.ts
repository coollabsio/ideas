import type { APIRoute } from 'astro';
import { getSession } from '~/lib/session';

export const prerender = false;

export const GET: APIRoute = ({ cookies }) => {
  const session = getSession(cookies.get('sid')?.value);
  if (!session) {
    return Response.json({ user: null });
  }
  return Response.json({
    user: { login: session.login, avatarUrl: session.avatarUrl },
    csrfToken: session.csrfToken,
  });
};
