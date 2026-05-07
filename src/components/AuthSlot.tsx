import * as React from 'react';
import { useEffect, useState } from 'react';
import { LogOut, Plus } from 'lucide-react';
import { Button } from './ui/button';
import { Avatar } from './ui/avatar';
import { NewIdeaDialog } from './NewIdeaDialog';

function GithubMark(props: React.SVGProps<SVGSVGElement>): React.ReactElement {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
      {...props}
    >
      <path d="M12 .5C5.6.5.5 5.7.5 12.1c0 5.1 3.3 9.5 7.9 11 .6.1.8-.2.8-.6v-2c-3.2.7-3.9-1.5-3.9-1.5-.5-1.3-1.3-1.7-1.3-1.7-1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1 1.8 2.7 1.3 3.4 1 .1-.8.4-1.3.8-1.6-2.6-.3-5.3-1.3-5.3-5.7 0-1.3.5-2.3 1.2-3.1-.1-.3-.5-1.5.1-3.1 0 0 1-.3 3.2 1.2.9-.3 1.9-.4 2.9-.4s2 .1 2.9.4c2.2-1.5 3.2-1.2 3.2-1.2.6 1.6.2 2.8.1 3.1.7.8 1.2 1.8 1.2 3.1 0 4.4-2.7 5.4-5.3 5.7.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6 4.6-1.5 7.9-5.9 7.9-11C23.5 5.7 18.4.5 12 .5z" />
    </svg>
  );
}

interface MeResponse {
  user: { login: string; avatarUrl: string } | null;
  csrfToken?: string;
}

declare global {
  interface Window {
    __ideasCsrfToken: string | null;
    __ideasUser: MeResponse['user'];
    __dispatchAuthReady?: () => void;
  }
}

export function AuthSlot(): React.ReactElement {
  const [me, setMe] = useState<MeResponse>({ user: null });
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);

  async function load(): Promise<void> {
    try {
      const res = await fetch('/api/me', { credentials: 'same-origin' });
      const data = (await res.json()) as MeResponse;
      setMe(data);
      window.__ideasUser = data.user;
      window.__ideasCsrfToken = data.csrfToken ?? null;
      window.dispatchEvent(new CustomEvent('auth:ready'));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
  }, []);

  async function logout(): Promise<void> {
    const csrfToken = window.__ideasCsrfToken;
    const res = await fetch('/api/auth/logout', {
      method: 'POST',
      credentials: 'same-origin',
      headers: {
        'content-type': 'application/json',
        ...(csrfToken ? { 'x-csrf-token': csrfToken } : {}),
      },
      body: '{}',
    });
    if (!res.ok) return;
    window.__ideasUser = null;
    window.__ideasCsrfToken = null;
    setMe({ user: null });
    window.dispatchEvent(new CustomEvent('auth:ready'));
  }

  function handleNewIdeaClick(): void {
    if (!me.user) {
      window.location.href = '/api/auth/login';
      return;
    }
    setDialogOpen(true);
  }

  if (loading) {
    return <div className="h-8 w-48 animate-pulse rounded-sm bg-coolgray-100" />;
  }

  return (
    <>
      <div className="flex items-center gap-2 sm:gap-3">
        <Button
          variant="highlighted"
          size="sm"
          onClick={handleNewIdeaClick}
          aria-label="New idea"
        >
          <Plus className="h-3.5 w-3.5" aria-hidden="true" />
          <span>New idea</span>
        </Button>

        {!me.user ? (
          <a
            href="/api/auth/login"
            className="inline-flex h-8 items-center justify-center gap-2 rounded-sm border-2 border-coolgray-300 bg-coolgray-100 px-2 text-sm font-medium text-white outline-0 transition-colors hover:bg-coolgray-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-warning focus-visible:ring-offset-2 focus-visible:ring-offset-base"
          >
            <GithubMark className="h-4 w-4" />
            <span>Sign in</span>
          </a>
        ) : (
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <Avatar src={me.user.avatarUrl} alt={me.user.login} fallback={me.user.login} size={28} />
              <span className="hidden text-sm font-medium text-white sm:inline">{me.user.login}</span>
            </div>
            <Button
              variant="ghost"
              size="icon"
              onClick={() => {
                void logout();
              }}
              aria-label="Sign out"
              title="Sign out"
            >
              <LogOut className="h-4 w-4" aria-hidden="true" />
            </Button>
          </div>
        )}
      </div>

      <NewIdeaDialog open={dialogOpen} onClose={() => setDialogOpen(false)} />
    </>
  );
}
