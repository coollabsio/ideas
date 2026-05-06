import type { APIRoute } from 'astro';
import { config } from '~/lib/config';
import { consumeOAuthState, createSession } from '~/lib/session';
import { fetchUser } from '~/lib/github';

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

  const tokenRes = await fetch('https://github.com/login/oauth/access_token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    body: JSON.stringify({
      client_id: config.githubClientId,
      client_secret: config.githubClientSecret,
      code,
      redirect_uri: `${config.baseUrl}/api/auth/callback`,
    }),
  });
  if (!tokenRes.ok) {
    return new Response('Token exchange failed', { status: 502 });
  }
  const tokenData = (await tokenRes.json()) as {
    access_token?: string;
    error?: string;
    error_description?: string;
  };
  if (!tokenData.access_token) {
    return new Response(`OAuth error: ${tokenData.error ?? 'no token'} ${tokenData.error_description ?? ''}`, {
      status: 502,
    });
  }

  const user = await fetchUser(tokenData.access_token);
  const session = createSession(
    tokenData.access_token,
    user.login,
    user.avatar_url,
    config.sessionTtlSec
  );

  cookies.set('sid', session.sid, {
    httpOnly: true,
    secure: config.baseUrl.startsWith('https'),
    sameSite: 'lax',
    path: '/',
    maxAge: config.sessionTtlSec,
  });

  return redirect('/', 302);
};
