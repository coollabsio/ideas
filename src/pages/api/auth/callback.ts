import type { APIRoute } from 'astro';
import { config } from '~/lib/config';
import { consumeOAuthState, createSession } from '~/lib/session';
import { exchangeCodeForToken, fetchViewer } from '~/lib/github';

export const prerender = false;

export const GET: APIRoute = async ({ url, cookies, redirect }) => {
  const code = url.searchParams.get('code');
  const state = url.searchParams.get('state');
  if (!code || !state) {
    return new Response('Missing code or state', { status: 400 });
  }
  if (!consumeOAuthState(state)) {
    return new Response('Invalid or expired state', { status: 400 });
  }

  let tokens;
  try {
    tokens = await exchangeCodeForToken(code);
  } catch (err) {
    return new Response(`Token exchange failed: ${(err as Error).message}`, { status: 502 });
  }

  let viewer;
  try {
    viewer = await fetchViewer(tokens.accessToken);
  } catch (err) {
    return new Response(`Failed to fetch user: ${(err as Error).message}`, { status: 502 });
  }

  const session = createSession(tokens, viewer.login, viewer.avatarUrl, config.sessionTtlSec);

  cookies.set('sid', session.sid, {
    httpOnly: true,
    secure: config.baseUrl.startsWith('https'),
    sameSite: 'lax',
    path: '/',
    maxAge: config.sessionTtlSec,
  });

  return redirect('/', 302);
};
