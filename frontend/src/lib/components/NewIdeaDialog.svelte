<script lang="ts">
  import { createIdea, type Idea } from '$lib/api';
  import { findSimilarIdeas, type SimilarIdeaMatch } from '$lib/similarity';

  export let open = false;
  export let csrfToken: string | null = null;
  export let ideas: Idea[] = [];
  export let onClose: () => void;
  export let onCreated: (idea: Idea) => void;
  export let onSelectSimilar: (idea: Idea) => void = () => {};

  let title = '';
  let body = '';
  let problem = '';
  let error = '';
  let submitting = false;

  const TITLE_MIN = 10;
  const TITLE_MAX = 300;
  const BODY_MIN = 30;
  const BODY_MAX = 10000;
  const PROBLEM_MIN = 30;
  const PROBLEM_MAX = 2000;

  $: titleLen = title.trim().length;
  $: bodyLen = body.trim().length;
  $: problemLen = problem.trim().length;
  $: canSubmit = Boolean(csrfToken) && titleLen >= TITLE_MIN && titleLen <= TITLE_MAX && bodyLen >= BODY_MIN && bodyLen <= BODY_MAX && problemLen >= PROBLEM_MIN && problemLen <= PROBLEM_MAX && !submitting;
  $: similarIdeas = findSimilarIdeas(title, body, ideas);

  function reset() {
    title = '';
    body = '';
    problem = '';
    error = '';
  }

  function close() {
    reset();
    onClose();
  }

  function selectSimilar(match: SimilarIdeaMatch) {
    reset();
    onSelectSimilar(match.idea);
  }

  function excerpt(idea: Idea) {
    return idea.bodyText.length > 120 ? `${idea.bodyText.slice(0, 120).trimEnd()}…` : idea.bodyText;
  }

  async function submit() {
    if (!canSubmit || !csrfToken) return;
    submitting = true;
    error = '';
    try {
      const idea = await createIdea(title.trim(), body.trim(), problem.trim(), csrfToken);
      onCreated(idea);
      close();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not create idea.';
    } finally {
      submitting = false;
    }
  }
</script>

{#if open}
  <div class="modal-backdrop" role="presentation" on:click={close}></div>
  <div class="modal" role="dialog" aria-modal="true" aria-labelledby="new-idea-title">
    <header>
      <div>
        <h2 id="new-idea-title">New idea</h2>
        <p>Submit a local idea. Your GitHub account is used only for identity.</p>
      </div>
      <button class="button button-ghost icon-button" type="button" on:click={close} aria-label="Close">×</button>
    </header>

    <form on:submit|preventDefault={submit}>
      {#if similarIdeas.length > 0}
        <section class="similar-ideas-callout" aria-labelledby="similar-ideas-title">
          <div>
            <strong id="similar-ideas-title">Are you looking for this?</strong>
            <p>This idea looks similar to existing ideas. You may want to upvote or comment there instead.</p>
          </div>
          <div class="similar-ideas-list">
            {#each similarIdeas as match (match.idea.id)}
              <button class="similar-idea" type="button" on:click={() => selectSimilar(match)}>
                <span class="similar-idea-title">{match.idea.title}</span>
                <span class="similar-idea-excerpt">{excerpt(match.idea)}</span>
                <span class="similar-idea-meta">{match.idea.status} · {match.idea.upvoteCount} upvotes</span>
              </button>
            {/each}
          </div>
        </section>
      {/if}

      <label>
        <span>Title <small>{titleLen}/{TITLE_MAX}</small></span>
        <input class="input" bind:value={title} maxlength={TITLE_MAX} required placeholder="A short, descriptive title" />
      </label>
      <label>
        <span>Body <small>{bodyLen}/{BODY_MAX}</small></span>
        <textarea class="input textarea" bind:value={body} maxlength={BODY_MAX} required rows="8" placeholder="Describe what this should do and why it is useful."></textarea>
      </label>
      <label>
        <span>What problem does this solve? <small>{problemLen}/{PROBLEM_MAX}</small></span>
        <textarea class="input textarea" bind:value={problem} maxlength={PROBLEM_MAX} required rows="5" placeholder="Explain the problem and why an alternative is needed — what is missing or insufficient in existing tools."></textarea>
      </label>
      {#if error}<p class="error">{error}</p>{/if}
      <footer>
        <button class="button button-ghost" type="button" on:click={close}>Cancel</button>
        <button class="button button-highlighted" type="submit" disabled={!canSubmit}>{submitting ? 'Creating…' : 'Create idea'}</button>
      </footer>
    </form>
  </div>
{/if}
