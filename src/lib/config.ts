import 'dotenv/config';

function need(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`Missing required env var: ${name}`);
  return v;
}

export const config = {
  get githubClientId() {
    return need('GITHUB_CLIENT_ID');
  },
  get githubClientSecret() {
    return need('GITHUB_CLIENT_SECRET');
  },
  get githubToken() {
    return need('GITHUB_TOKEN');
  },
  get githubLoginEnabled() {
    return process.env.GITHUB_LOGIN_ENABLED !== 'false';
  },
  get baseUrl() {
    return process.env.PUBLIC_BASE_URL ?? 'http://localhost:4321';
  },
  repo: { owner: 'coollabsio', name: 'ideas' },
  ideasCategory: 'Ideas',
  sessionTtlSec: 7 * 24 * 3600,
};
