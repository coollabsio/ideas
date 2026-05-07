interface IdeaClient {
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

function setUpvoteButtonsBusy(busy: boolean): void {
  const shouldDisable = busy && Boolean(window.__ideasUser);
  document.querySelectorAll<HTMLButtonElement>('button.upvote').forEach((btn) => {
    btn.disabled = shouldDisable;
    btn.setAttribute('aria-busy', busy ? 'true' : 'false');
  });
}

function setButtonDisabled(btn: HTMLButtonElement, disabled: boolean): void {
  btn.disabled = disabled;
  btn.setAttribute('aria-busy', disabled ? 'true' : 'false');
}

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

function githubLoginEnabled(): boolean {
  return window.__githubLoginEnabled !== false;
}

function notifyLoginDisabled(): void {
  window.alert('GitHub sign-in is temporarily disabled.');
}

async function refreshIdeas(): Promise<void> {
  setUpvoteButtonsBusy(true);
  try {
    const res = await fetch('/api/issues', { credentials: 'same-origin' });
    if (!res.ok) return;
    const ideas = (await res.json()) as IdeaClient[];
    reconcileIdeas(ideas);
  } catch (err) {
    console.error('issues refresh failed', err);
  } finally {
    setUpvoteButtonsBusy(false);
  }
}

async function handleUpvote(e: Event): Promise<void> {
  const btn = e.currentTarget as HTMLButtonElement;
  const user = window.__ideasUser ?? null;
  const csrfToken = window.__ideasCsrfToken ?? null;
  if (!user) {
    if (!githubLoginEnabled()) {
      notifyLoginDisabled();
      return;
    }
    location.href = '/api/auth/login';
    return;
  }
  if (!csrfToken) return;
  const issueNumber = Number.parseInt(btn.dataset.issueNumber ?? '', 10);
  if (!Number.isInteger(issueNumber)) return;
  const upvoted = btn.dataset.upvoted === '1';
  setButtonDisabled(btn, true);
  try {
    const res = await fetch('/api/upvote', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'content-type': 'application/json', 'x-csrf-token': csrfToken },
      body: JSON.stringify({ issueNumber, upvoted }),
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
    void refreshIdeas();
  } catch (err) {
    console.error('upvote failed', err);
  } finally {
    setButtonDisabled(btn, false);
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

function buildIdeaCard(idea: IdeaClient): HTMLElement {
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
  const closedBadge = idea.closed
    ? '<span class="ml-2 inline-block rounded-full border border-error/50 bg-error/10 px-2 py-0.5 align-middle text-xs font-bold uppercase tracking-wider text-error">Closed</span>'
    : '';

  article.innerHTML = `
    <button
      type="button"
      class="upvote relative z-10 shrink-0 w-12 h-14 rounded-sm border flex flex-col items-center justify-center transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-coollabs dark:focus-visible:ring-warning focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-base disabled:cursor-not-allowed disabled:opacity-50 ${upvotedClasses}"
      data-issue-number="${idea.number}"
      data-upvoted="${idea.viewerHasUpvoted ? '1' : '0'}"
      aria-label="${idea.viewerHasUpvoted ? 'Remove upvote' : 'Upvote'}"
      aria-busy="false"
    >
      <svg aria-hidden="true" viewBox="0 0 24 24" class="upvote-icon h-3 w-3" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
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
      >${escapeHtml(idea.title)}${closedBadge}</a>
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

function renderCards(container: HTMLElement, ideas: IdeaClient[]): void {
  const cards = ideas.map((idea) => {
    const card = buildIdeaCard(idea);
    const btn = card.querySelector<HTMLButtonElement>('button.upvote');
    if (btn) attachUpvoteHandler(btn);
    return card;
  });
  container.replaceChildren(...cards);
}

function ensureClosedIdeasList(closedCount: number): HTMLElement | null {
  let list = document.getElementById('closed-ideas-list');
  if (list || closedCount === 0) return list;

  const openList = document.getElementById('ideas-list');
  if (!openList?.parentElement) return null;

  const details = document.createElement('details');
  details.className = 'group mt-6';
  details.innerHTML = `
    <summary class="flex cursor-pointer list-none items-center gap-2 border-t border-neutral-200 pt-4 text-sm font-bold uppercase tracking-widest text-neutral-500 hover:text-neutral-700 dark:border-coolgray-200 dark:hover:text-neutral-300">
      <span class="text-neutral-600 transition-transform group-open:rotate-90">▸</span>
      Closed ideas
      <span data-closed-count class="font-mono text-xs text-neutral-600">(0)</span>
    </summary>
    <div id="closed-ideas-list" class="mt-3 space-y-2"></div>
  `;
  openList.insertAdjacentElement('afterend', details);
  list = document.getElementById('closed-ideas-list');
  return list;
}

function updateStats(ideas: IdeaClient[]): void {
  const openCount = ideas.filter((idea) => !idea.closed).length;
  const totalUpvotes = ideas.reduce((acc, idea) => acc + idea.upvoteCount, 0);
  const ideasEl = document.querySelector<HTMLElement>('[data-stat="ideas"]');
  const upvotesEl = document.querySelector<HTMLElement>('[data-stat="upvotes"]');
  const closedCountEl = document.querySelector<HTMLElement>('[data-closed-count]');
  if (ideasEl) ideasEl.textContent = String(openCount);
  if (upvotesEl) upvotesEl.textContent = String(totalUpvotes);
  if (closedCountEl) {
    const closedCount = ideas.length - openCount;
    closedCountEl.textContent = `(${closedCount})`;
  }
}

function reconcileIdeas(ideas: IdeaClient[]): void {
  const openList = document.getElementById('ideas-list');
  if (!openList) return;

  const openIdeas = ideas.filter((idea) => !idea.closed);
  const closedIdeas = ideas.filter((idea) => idea.closed);
  renderCards(openList, openIdeas);

  const closedList = ensureClosedIdeasList(closedIdeas.length);
  if (closedList) {
    renderCards(closedList, closedIdeas);
    const details = closedList.closest('details');
    if (details instanceof HTMLElement) details.hidden = closedIdeas.length === 0;
  }

  updateStats(ideas);
}

function handleIdeaCreated(e: Event): void {
  const idea = (e as CustomEvent<IdeaClient>).detail;
  if (!idea) return;
  const list = document.getElementById(idea.closed ? 'closed-ideas-list' : 'ideas-list');
  if (list && !list.querySelector<HTMLElement>(`[data-id="${CSS.escape(idea.id)}"]`)) {
    const card = buildIdeaCard(idea);
    list.prepend(card);
    const btn = card.querySelector<HTMLButtonElement>('button.upvote');
    if (btn) attachUpvoteHandler(btn);
    card.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }
  void refreshIdeas();
}

function init(): void {
  setUpvoteButtonsBusy(true);
  document.querySelectorAll<HTMLButtonElement>('button.upvote').forEach(attachUpvoteHandler);
  window.addEventListener('auth:ready', () => {
    void refreshIdeas();
  });
  window.addEventListener('idea:created', handleIdeaCreated);
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init, { once: true });
} else {
  init();
}

export {};
