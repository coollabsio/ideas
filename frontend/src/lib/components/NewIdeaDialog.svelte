<script lang="ts">
  import { createIdea, type Idea } from '$lib/api';

  export let open = false;
  export let csrfToken: string | null = null;
  export let onClose: () => void;
  export let onCreated: (idea: Idea) => void;

  let title = '';
  let body = '';
  let error = '';
  let submitting = false;

  const TITLE_MIN = 10;
  const TITLE_MAX = 300;
  const BODY_MIN = 30;
  const BODY_MAX = 10000;

  $: titleLen = title.trim().length;
  $: bodyLen = body.trim().length;
  $: canSubmit = Boolean(csrfToken) && titleLen >= TITLE_MIN && titleLen <= TITLE_MAX && bodyLen >= BODY_MIN && bodyLen <= BODY_MAX && !submitting;

  function reset() {
    title = '';
    body = '';
    error = '';
  }

  function close() {
    reset();
    onClose();
  }

  async function submit() {
    if (!canSubmit || !csrfToken) return;
    submitting = true;
    error = '';
    try {
      const idea = await createIdea(title.trim(), body.trim(), csrfToken);
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
      <button class="icon-button" type="button" on:click={close} aria-label="Close">×</button>
    </header>

    <form on:submit|preventDefault={submit}>
      <label>
        <span>Title <small>{titleLen}/{TITLE_MAX}</small></span>
        <input class="input" bind:value={title} maxlength={TITLE_MAX} required placeholder="A short, descriptive title" />
      </label>
      <label>
        <span>Body <small>{bodyLen}/{BODY_MAX}</small></span>
        <textarea class="input textarea" bind:value={body} maxlength={BODY_MAX} required rows="8" placeholder="Describe what this should do and why it is useful."></textarea>
      </label>
      {#if error}<p class="error">{error}</p>{/if}
      <footer>
        <button class="button button-ghost" type="button" on:click={close}>Cancel</button>
        <button class="button button-highlighted" type="submit" disabled={!canSubmit}>{submitting ? 'Creating…' : 'Create idea'}</button>
      </footer>
    </form>
  </div>
{/if}
