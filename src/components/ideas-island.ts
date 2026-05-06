interface IdeaSummary {
  id: string;
  upvoteCount: number;
  viewerHasUpvoted: boolean;
}

const ACTIVE = ['border-warning/50', 'bg-warning/15', 'text-warning'];
const INACTIVE = [
  'border-coolgray-300',
  'bg-base',
  'text-neutral-400',
  'hover:border-warning/40',
  'hover:text-warning',
];

function applyStyle(btn: HTMLButtonElement): void {
  const upvoted = btn.dataset.upvoted === '1';
  if (upvoted) {
    btn.classList.add(...ACTIVE);
    btn.classList.remove(...INACTIVE);
    btn.setAttribute('aria-label', 'Remove upvote');
  } else {
    btn.classList.remove(...ACTIVE);
    btn.classList.add(...INACTIVE);
    btn.setAttribute('aria-label', 'Upvote');
  }
}

async function refreshIdeas(): Promise<void> {
  try {
    const res = await fetch('/api/discussions', { credentials: 'same-origin' });
    if (!res.ok) return;
    const ideas = (await res.json()) as IdeaSummary[];
    for (const idea of ideas) {
      const btn = document.querySelector<HTMLButtonElement>(
        `button.upvote[data-discussion-id="${CSS.escape(idea.id)}"]`
      );
      if (!btn) continue;
      const countEl = btn.querySelector<HTMLElement>('.count');
      if (countEl) countEl.textContent = String(idea.upvoteCount);
      btn.dataset.upvoted = idea.viewerHasUpvoted ? '1' : '0';
      applyStyle(btn);
    }
  } catch (err) {
    console.error('discussions refresh failed', err);
  }
}

async function handleUpvote(e: Event): Promise<void> {
  const btn = e.currentTarget as HTMLButtonElement;
  const user = window.__ideasUser ?? null;
  const csrfToken = window.__ideasCsrfToken ?? null;
  if (!user) {
    location.href = '/api/auth/login';
    return;
  }
  if (!csrfToken) return;
  const id = btn.dataset.discussionId;
  if (!id) return;
  const upvoted = btn.dataset.upvoted === '1';
  btn.disabled = true;
  try {
    const res = await fetch('/api/upvote', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'content-type': 'application/json', 'x-csrf-token': csrfToken },
      body: JSON.stringify({ discussionId: id, upvoted }),
    });
    if (res.status === 401) {
      location.href = '/api/auth/login';
      return;
    }
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    const data = (await res.json()) as { upvoteCount: number; viewerHasUpvoted: boolean };
    const countEl = btn.querySelector<HTMLElement>('.count');
    if (countEl) countEl.textContent = String(data.upvoteCount);
    btn.dataset.upvoted = data.viewerHasUpvoted ? '1' : '0';
    applyStyle(btn);
  } catch (err) {
    console.error('upvote failed', err);
  } finally {
    btn.disabled = false;
  }
}

function init(): void {
  document.querySelectorAll<HTMLButtonElement>('button.upvote').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      void handleUpvote(e);
    });
    applyStyle(btn);
  });
  window.addEventListener('auth:ready', () => {
    void refreshIdeas();
  });
  void refreshIdeas();
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init, { once: true });
} else {
  init();
}

export {};
