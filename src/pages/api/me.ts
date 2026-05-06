import type { APIRoute } from 'astro';
import { getSession } from '~/lib/session';

export const prerender = false;

export const GET: APIRoute = ({ cookies }) => {
  const session = getSession(cookies.get('sid')?.value);
  const headers = { 'cache-control': 'no-store' };
  if (!session) {
    return new Response(JSON.stringify({ user: null }), {
      headers: { ...headers, 'content-type': 'application/json' },
    });
  }
  return new Response(
    JSON.stringify({
      user: { login: session.login, avatarUrl: session.avatarUrl },
      csrfToken: session.csrfToken,
    }),
    { headers: { ...headers, 'content-type': 'application/json' } }
  );
};
