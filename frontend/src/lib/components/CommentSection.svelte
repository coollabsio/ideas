<script lang="ts">
  import { createComment, deleteComment, fetchComments, updateComment, type Comment } from '$lib/api';

  export let ideaId: string | null = null;
  export let csrfToken: string | null = null;
  export let onCommentCountChange: (delta: number) => void = () => {};

  const COMMENT_MAX = 2000;

  let comments: Comment[] = [];
  let body = '';
  let error = '';
  let loading = false;
  let busy = false;
  let loadedIdeaId: string | null = null;
  let editingId: string | null = null;
  let editingBody = '';

  $: bodyLen = body.trim().length;
  $: editingLen = editingBody.trim().length;
  $: canSubmit = Boolean(csrfToken && ideaId && bodyLen > 0 && bodyLen <= COMMENT_MAX && !busy);
  $: canSaveEdit = Boolean(csrfToken && editingId && editingLen > 0 && editingLen <= COMMENT_MAX && !busy);

  $: if (ideaId && ideaId !== loadedIdeaId) {
    loadComments(ideaId);
  }

  $: if (!ideaId) {
    comments = [];
    loadedIdeaId = null;
    loading = false;
    error = '';
  }

  async function loadComments(nextIdeaId: string) {
    loadedIdeaId = nextIdeaId;
    loading = true;
    error = '';
    try {
      comments = await fetchComments(nextIdeaId);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not load comments.';
    } finally {
      loading = false;
    }
  }

  async function submit() {
    if (!ideaId || !csrfToken || !canSubmit) return;
    busy = true;
    error = '';
    try {
      const comment = await createComment(ideaId, body.trim(), csrfToken);
      comments = [...comments, comment];
      body = '';
      onCommentCountChange(1);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not add comment.';
    } finally {
      busy = false;
    }
  }

  function startEdit(comment: Comment) {
    editingId = comment.id;
    editingBody = comment.bodyText;
    error = '';
  }

  function cancelEdit() {
    editingId = null;
    editingBody = '';
  }

  async function saveEdit(comment: Comment) {
    if (!csrfToken || !canSaveEdit) return;
    busy = true;
    error = '';
    try {
      const updated = await updateComment(comment.id, editingBody.trim(), csrfToken);
      comments = comments.map((item) => (item.id === updated.id ? updated : item));
      cancelEdit();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not update comment.';
    } finally {
      busy = false;
    }
  }

  async function remove(comment: Comment) {
    if (!csrfToken || busy) return;
    if (!confirm('Delete this comment?')) return;
    busy = true;
    error = '';
    try {
      await deleteComment(comment.id, csrfToken);
      comments = comments.filter((item) => item.id !== comment.id);
      onCommentCountChange(-1);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not delete comment.';
    } finally {
      busy = false;
    }
  }

  function formatDate(value: string) {
    return new Date(value).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }
</script>

<section class="comments-section" aria-labelledby="comments-heading">
  <header class="comments-head">
    <h3 id="comments-heading">Comments <small>({comments.length})</small></h3>
    <span>{csrfToken ? 'Logged-in discussion' : 'Read-only for guests'}</span>
  </header>

  {#if error}<p class="error">{error}</p>{/if}

  {#if loading}
    <p class="comments-empty">Loading comments…</p>
  {:else if comments.length === 0}
    <p class="comments-empty">No comments yet. Sign in to start the discussion.</p>
  {:else}
    <div class="comments-list">
      {#each comments as comment (comment.id)}
        <article class="comment-card">
          <header>
            <img src={comment.author.avatarUrl} alt="" />
            <div>
              <strong>{comment.author.login}</strong>
              <time datetime={comment.createdAt}>{formatDate(comment.createdAt)}</time>
            </div>
          </header>

          {#if editingId === comment.id}
            <label class="comment-editor-label">
              <span>Edit comment <small>{editingLen}/{COMMENT_MAX}</small></span>
              <textarea class="input textarea comment-textarea" bind:value={editingBody} maxlength={COMMENT_MAX} rows="4"></textarea>
            </label>
            <footer class="comment-actions">
              <button class="button button-ghost" type="button" disabled={busy} on:click={cancelEdit}>Cancel</button>
              <button class="button button-highlighted" type="button" disabled={!canSaveEdit} on:click={() => saveEdit(comment)}>{busy ? 'Saving…' : 'Save'}</button>
            </footer>
          {:else}
            <p>{comment.bodyText}</p>
            {#if comment.viewerCanEdit || comment.viewerCanDelete}
              <footer class="comment-actions">
                {#if comment.viewerCanEdit}<button class="button button-ghost" type="button" disabled={busy} on:click={() => startEdit(comment)}>Edit</button>{/if}
                {#if comment.viewerCanDelete}<button class="button button-danger" type="button" disabled={busy} on:click={() => remove(comment)}>Delete</button>{/if}
              </footer>
            {/if}
          {/if}
        </article>
      {/each}
    </div>
  {/if}

  {#if csrfToken}
    <form class="comment-form" on:submit|preventDefault={submit}>
      <label>
        <span>Add comment <small>{bodyLen}/{COMMENT_MAX}</small></span>
        <textarea class="input textarea comment-textarea" bind:value={body} maxlength={COMMENT_MAX} rows="4" placeholder="Share feedback, questions, or implementation notes."></textarea>
      </label>
      <div class="comment-submit-row">
        <button class="button button-highlighted" type="submit" disabled={!canSubmit}>{busy ? 'Posting…' : 'Post comment'}</button>
      </div>
    </form>
  {:else}
    <p class="comments-signin">Sign in with GitHub to add a comment.</p>
  {/if}
</section>
