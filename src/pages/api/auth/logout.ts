import type { APIRoute } from 'astro';
import { deleteSession } from '~/lib/session';

export const prerender = false;

export const POST: APIRoute = ({ cookies }) => {
  const sid = cookies.get('sid')?.value;
  if (sid) deleteSession(sid);
  cookies.delete('sid', { path: '/' });
  return new Response(null, { status: 204 });
};
