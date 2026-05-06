import type { APIRoute } from 'astro';
import { config } from '~/lib/config';
import { createOAuthState } from '~/lib/session';

export const prerender = false;

export const GET: APIRoute = () => {
  const state = createOAuthState();
  const url = new URL('https://github.com/login/oauth/authorize');
  url.searchParams.set('client_id', config.githubClientId);
  url.searchParams.set('state', state);
  url.searchParams.set('redirect_uri', `${config.baseUrl}/api/auth/callback`);
  return Response.redirect(url.toString(), 302);
};
