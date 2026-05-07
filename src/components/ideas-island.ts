interface IdeaSummary {
  id: string;
  upvoteCount: number;
  viewerHasUpvoted: boolean;
}

interface CreatedIdea {
  id: string;
  number: number;
  title: string;
  bodyText: string;
  url: string;
  upvoteCount: number;
  viewerHasUpvoted: boolean;
  author: { login: string; avatarUrl: string } | null;
  createdAt: string;
  closed: boolean;
}

const ACTIVE = [
  'border-coollabs',
  'bg-coollabs-50',
  'text-coollabs-200',
  'dark:border-warning/50',
  'dark:bg-warning/15',
  'dark:text-warning',
];
const INACTIVE = [
  'border-neutral-200',
  'bg-white',
  'text-neutral-500',
  'hover:border-coollabs/40',
  'hover:text-coollabs',
  'dark:border-coolgray-300',
  'dark:bg-base',
  'dark:text-neutral-400',
  'dark:hover:border-warning/40',
  'dark:hover:text-warning',
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

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function attachUpvoteHandler(btn: HTMLButtonElement): void {
  btn.addEventListener('click', (e) => {
    void handleUpvote(e);
  });
  applyStyle(btn);
}

function buildIdeaCard(idea: CreatedIdea): HTMLElement {
  const excerpt =
    idea.bodyText.length > 240 ? idea.bodyText.slice(0, 240).trimEnd() + '…' : idea.bodyText;
  const date = new Date(idea.createdAt).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });

  const article = document.createElement('article');
  article.className =
    'idea-card coolbox group flex-row gap-3 p-3';
  article.dataset.id = idea.id;

  const upvotedClasses = idea.viewerHasUpvoted
    ? 'border-coollabs bg-coollabs-50 text-coollabs-200 dark:border-warning/50 dark:bg-warning/15 dark:text-warning'
    : 'border-neutral-200 bg-white text-neutral-500 hover:border-coollabs/40 hover:text-coollabs dark:border-coolgray-300 dark:bg-base dark:text-neutral-400 dark:hover:border-warning/40 dark:hover:text-warning';

  article.innerHTML = `
    <button
      type="button"
      class="upvote relative z-10 shrink-0 w-12 h-14 rounded-sm border flex flex-col items-center justify-center transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-coollabs dark:focus-visible:ring-warning focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-base disabled:cursor-not-allowed disabled:opacity-50 ${upvotedClasses}"
      data-discussion-id="${escapeHtml(idea.id)}"
      data-upvoted="${idea.viewerHasUpvoted ? '1' : '0'}"
      aria-label="${idea.viewerHasUpvoted ? 'Remove upvote' : 'Upvote'}"
    >
      <svg aria-hidden="true" viewBox="0 0 24 24" class="h-3 w-3" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 5l-7 7M12 5l7 7M12 5v14"/>
      </svg>
      <span class="count mt-1 font-mono text-sm font-bold leading-none">${idea.upvoteCount}</span>
    </button>
    <div class="min-w-0 flex-1">
      <a
        href="${escapeHtml(idea.url)}"
        target="_blank"
        rel="noopener"
        class="box-title block text-base font-bold leading-snug hover:text-coollabs focus-visible:outline-none focus-visible:text-coollabs dark:hover:text-warning dark:focus-visible:text-warning before:absolute before:inset-0 before:content-['']"
      >${escapeHtml(idea.title)}</a>
      <p class="box-description mt-1 line-clamp-2">${escapeHtml(excerpt)}</p>
      <div class="mt-2 flex items-center gap-2 text-xs uppercase tracking-wide text-neutral-500">
        ${
          idea.author
            ? `<img src="${escapeHtml(idea.author.avatarUrl)}" alt="" class="h-4 w-4 rounded-full border border-neutral-200 dark:border-coolgray-300" /><span class="font-bold normal-case">${escapeHtml(idea.author.login)}</span><span aria-hidden="true">·</span>`
            : ''
        }
        <time datetime="${escapeHtml(idea.createdAt)}" class="font-mono normal-case">${escapeHtml(date)}</time>
        <span aria-hidden="true">·</span>
        <a href="${escapeHtml(idea.url)}" target="_blank" rel="noopener" class="relative z-10 font-mono normal-case hover:text-warning">#${idea.number}</a>
      </div>
    </div>
  `;
  return article;
}

function bumpIdeasStat(): void {
  const el = document.querySelector<HTMLElement>('[data-stat="ideas"]');
  if (!el) return;
  const n = Number.parseInt(el.textContent ?? '0', 10);
  if (Number.isFinite(n)) el.textContent = String(n + 1);
}

function handleIdeaCreated(e: Event): void {
  const idea = (e as CustomEvent<CreatedIdea>).detail;
  if (!idea) return;
  const list = document.getElementById('ideas-list');
  if (!list) return;
  const existing = list.querySelector<HTMLElement>(`[data-id="${CSS.escape(idea.id)}"]`);
  if (existing) return;
  const card = buildIdeaCard(idea);
  list.prepend(card);
  const btn = card.querySelector<HTMLButtonElement>('button.upvote');
  if (btn) attachUpvoteHandler(btn);
  bumpIdeasStat();
  card.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
}

function init(): void {
  document.querySelectorAll<HTMLButtonElement>('button.upvote').forEach(attachUpvoteHandler);
  window.addEventListener('auth:ready', () => {
    void refreshIdeas();
  });
  window.addEventListener('idea:created', handleIdeaCreated);
  void refreshIdeas();
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init, { once: true });
} else {
  init();
}

export {};
