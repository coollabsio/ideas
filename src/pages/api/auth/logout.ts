import type { APIRoute } from 'astro';
import { config } from '~/lib/config';
import { deleteSession, getSession } from '~/lib/session';

export const prerender = false;

export const POST: APIRoute = ({ request, cookies }) => {
  const sid = cookies.get('sid')?.value;
  const session = getSession(sid);

  if (session) {
    const csrfHeader = request.headers.get('x-csrf-token');
    if (csrfHeader !== session.csrfToken) {
      return new Response('Invalid CSRF token', { status: 403 });
    }
    deleteSession(session.sid);
  }

  cookies.set('sid', '', {
    httpOnly: true,
    secure: config.baseUrl.startsWith('https'),
    sameSite: 'lax',
    path: '/',
    maxAge: 0,
    expires: new Date(0),
  });

  return new Response(null, {
    status: 204,
    headers: { 'cache-control': 'no-store' },
  });
};
