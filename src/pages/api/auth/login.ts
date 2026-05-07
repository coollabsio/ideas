import type { APIRoute } from 'astro';
import { config } from '~/lib/config';
import { createOAuthState } from '~/lib/session';

export const prerender = false;

export const GET: APIRoute = ({ redirect }) => {
  if (!config.githubLoginEnabled) {
    return new Response('GitHub login is temporarily disabled.', { status: 503 });
  }

  const state = createOAuthState();
  const url = new URL('https://github.com/login/oauth/authorize');
  url.searchParams.set('client_id', config.githubClientId);
  url.searchParams.set('scope', 'public_repo');
  url.searchParams.set('state', state);
  url.searchParams.set('redirect_uri', `${config.baseUrl}/api/auth/callback`);
  url.searchParams.set('allow_signup', 'true');
  return redirect(url.toString(), 302);
};
