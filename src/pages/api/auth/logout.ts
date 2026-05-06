import type { APIRoute } from 'astro';
import { config } from '~/lib/config';
import { deleteSession } from '~/lib/session';

export const prerender = false;

export const POST: APIRoute = ({ cookies }) => {
  const sid = cookies.get('sid')?.value;
  if (sid) deleteSession(sid);

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
