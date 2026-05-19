<script lang="ts">
  import { deleteIdea, setIdeaStatus, updateIdea, type Idea, type IdeaStatus } from '$lib/api';
  import CommentSection from '$lib/components/CommentSection.svelte';

  export let idea: Idea | null = null;
  export let csrfToken: string | null = null;
  export let onClose: () => void;
  export let onUpdated: (idea: Idea) => void;
  export let onDeleted: (idea: Idea) => void;

  let title = '';
  let body = '';
  let problem = '';
  let error = '';
  let busy = false;
  let loadedIdeaId: string | null = null;

  const TITLE_MIN = 10;
  const TITLE_MAX = 300;
  const BODY_MIN = 30;
  const BODY_MAX = 10000;
  const PROBLEM_MIN = 30;
  const PROBLEM_MAX = 2000;

  $: if (idea && idea.id !== loadedIdeaId) {
    loadedIdeaId = idea.id;
    title = idea.title;
    body = idea.bodyText;
    problem = idea.problem;
    error = '';
    busy = false;
  }
  $: titleLen = title.trim().length;
  $: bodyLen = body.trim().length;
  $: problemLen = problem.trim().length;
  $: canEdit = Boolean(idea?.viewerCanEdit && csrfToken);
  $: canDelete = Boolean(idea?.viewerCanDelete && csrfToken);
  $: canClose = Boolean(idea?.viewerCanClose && csrfToken);
  $: canManage = canEdit || canDelete || canClose;
  $: canSave = canEdit && titleLen >= TITLE_MIN && titleLen <= TITLE_MAX && bodyLen >= BODY_MIN && bodyLen <= BODY_MAX && problemLen >= PROBLEM_MIN && problemLen <= PROBLEM_MAX && !busy;
  $: createdDate = idea ? new Date(idea.createdAt).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' }) : '';
  $: updatedDate = idea ? new Date(idea.updatedAt).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' }) : '';

  function close() {
    loadedIdeaId = null;
    error = '';
    onClose();
  }

  async function save() {
    if (!idea || !canSave || !csrfToken) return;
    busy = true;
    error = '';
    try {
      const updated = await updateIdea(idea.id, title.trim(), body.trim(), problem.trim(), csrfToken);
      onUpdated(updated);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not update idea.';
    } finally {
      busy = false;
    }
  }

  async function updateStatus(status: IdeaStatus) {
    if (!idea || !canClose || !csrfToken || busy || idea.status === status) return;
    busy = true;
    error = '';
    try {
      const updated = await setIdeaStatus(idea.id, status, csrfToken);
      onUpdated(updated);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not update idea status.';
    } finally {
      busy = false;
    }
  }

  async function remove() {
    if (!idea || !canDelete || !csrfToken || busy) return;
    if (!confirm('Delete this idea? This cannot be undone.')) return;
    busy = true;
    error = '';
    try {
      await deleteIdea(idea.id, csrfToken);
      onDeleted(idea);
      close();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not delete idea.';
    } finally {
      busy = false;
    }
  }

  function updateCommentCount(delta: number) {
    if (!idea) return;
    onUpdated({ ...idea, commentCount: Math.max(0, idea.commentCount + delta) });
  }
</script>

{#if idea}
  <div class="modal-backdrop" role="presentation" on:click={close}></div>
  <div class="modal idea-details-modal" role="dialog" aria-modal="true" aria-labelledby="idea-details-title">
    <header>
      <div>
        <h2 id="idea-details-title">{canEdit ? 'Edit idea' : canManage ? 'Manage idea' : 'Idea details'}</h2>
        <p>
          By <strong>{idea.author.login}</strong> · {createdDate}
          {#if idea.updatedAt !== idea.createdAt} · updated {updatedDate}{/if}
        </p>
      </div>
      <button class="button button-ghost icon-button" type="button" on:click={close} aria-label="Close">×</button>
    </header>

    {#if canEdit}
      <form on:submit|preventDefault={save}>
        <label>
          <span>Title <small>{titleLen}/{TITLE_MAX}</small></span>
          <input class="input" bind:value={title} maxlength={TITLE_MAX} required />
        </label>
        <label>
          <span>Body <small>{bodyLen}/{BODY_MAX}</small></span>
          <textarea class="input textarea" bind:value={body} maxlength={BODY_MAX} required rows="10"></textarea>
        </label>
        <label>
          <span>What problem does this solve? <small>{problemLen}/{PROBLEM_MAX}</small></span>
          <textarea class="input textarea" bind:value={problem} maxlength={PROBLEM_MAX} required rows="5" placeholder="Explain the problem and why an alternative is needed — what is missing or insufficient in existing tools."></textarea>
        </label>
        {#if error}<p class="error">{error}</p>{/if}
        <footer class="modal-actions">
          <div class="secondary-actions">
            {#if canClose}
              {#if idea.status === 'open'}
                <button class="button button-ghost" type="button" disabled={busy} on:click={() => updateStatus('inprogress')}>Mark in progress</button>
                <button class="button button-ghost" type="button" disabled={busy} on:click={() => updateStatus('closed')}>Close idea</button>
              {:else if idea.status === 'inprogress'}
                <button class="button button-ghost" type="button" disabled={busy} on:click={() => updateStatus('open')}>Reopen</button>
                <button class="button button-ghost" type="button" disabled={busy} on:click={() => updateStatus('closed')}>Close idea</button>
              {:else}
                <button class="button button-ghost" type="button" disabled={busy} on:click={() => updateStatus('open')}>Reopen</button>
              {/if}
            {/if}
            {#if canDelete}<button class="button button-danger" type="button" disabled={busy} on:click={remove}>Delete</button>{/if}
          </div>
          <div class="primary-actions">
            <button class="button button-ghost" type="button" on:click={close}>Cancel</button>
            <button class="button button-highlighted" type="submit" disabled={!canSave}>{busy ? 'Saving…' : 'Save changes'}</button>
          </div>
        </footer>
      </form>
    {:else}
      <div class="idea-details-readonly">
        <div class="idea-details-status">
          <span class="count">{idea.upvoteCount} upvotes</span>
          {#if idea.status === 'inprogress'}<span class="inprogress-badge">In progress</span>{/if}
          {#if idea.status === 'closed'}<span class="closed-badge">Closed</span>{/if}
        </div>
        <h3>{idea.title}</h3>
        <p>{idea.bodyText}</p>
        {#if idea.problem}
          <div class="idea-problem">
            <strong>What problem does this solve?</strong>
            <p>{idea.problem}</p>
          </div>
        {/if}
        {#if error}<p class="error">{error}</p>{/if}
        {#if canDelete || canClose}
          <footer class="modal-actions">
            <div class="secondary-actions">
              {#if canClose}
                {#if idea.status === 'open'}
                  <button class="button button-ghost" type="button" disabled={busy} on:click={() => updateStatus('inprogress')}>Mark in progress</button>
                  <button class="button button-ghost" type="button" disabled={busy} on:click={() => updateStatus('closed')}>Close idea</button>
                {:else if idea.status === 'inprogress'}
                  <button class="button button-ghost" type="button" disabled={busy} on:click={() => updateStatus('open')}>Reopen</button>
                  <button class="button button-ghost" type="button" disabled={busy} on:click={() => updateStatus('closed')}>Close idea</button>
                {:else}
                  <button class="button button-ghost" type="button" disabled={busy} on:click={() => updateStatus('open')}>Reopen</button>
                {/if}
              {/if}
              {#if canDelete}<button class="button button-danger" type="button" disabled={busy} on:click={remove}>Delete</button>{/if}
            </div>
            <div class="primary-actions">
              <button class="button button-ghost" type="button" on:click={close}>Cancel</button>
            </div>
          </footer>
        {/if}
      </div>
    {/if}

    <CommentSection ideaId={idea.id} {csrfToken} onCommentCountChange={updateCommentCount} />
  </div>
{/if}

<style>
  .idea-problem {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border, rgba(255, 255, 255, 0.12));
  }
  .idea-problem strong {
    display: block;
    margin-bottom: 0.35rem;
  }
  .idea-problem p {
    margin: 0;
  }
</style>
