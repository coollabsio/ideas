<script lang="ts">
  import type { Idea } from '$lib/api';

  export let idea: Idea;
  export let canVote = false;
  export let busy = false;
  export let onToggle: (idea: Idea) => void;
  export let onOpen: (idea: Idea) => void;

  $: excerpt = idea.bodyText.length > 240 ? `${idea.bodyText.slice(0, 240).trimEnd()}…` : idea.bodyText;
  $: date = new Date(idea.createdAt).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onOpen(idea);
    }
  }
</script>

<div class="idea-card coolbox group" class:inprogress={idea.status === 'inprogress'} class:done={idea.status === 'done'} class:closed={idea.closed} role="button" tabindex="0" on:click={() => onOpen(idea)} on:keydown={handleKeydown} aria-label={`Open idea: ${idea.title}`}>
  <button
    class:active={idea.viewerHasUpvoted}
    class="upvote"
    type="button"
    disabled={busy}
    aria-label={idea.viewerHasUpvoted ? 'Remove upvote' : canVote ? 'Upvote' : 'Sign in to upvote'}
    on:click|stopPropagation={() => onToggle(idea)}
  >
    <svg aria-hidden="true" viewBox="0 0 24 24" class="upvote-icon" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
      <path d="M12 5l-7 7M12 5l7 7M12 5v14" />
    </svg>
    <span class="count">{idea.upvoteCount}</span>
  </button>

  <div class="idea-main">
    <h3 class="box-title">
      {idea.title}
      {#if idea.status === 'inprogress'}<span class="inprogress-badge">In progress</span>{/if}
      {#if idea.status === 'done'}<span class="done-badge">Done</span>{/if}
      {#if idea.status === 'closed'}<span class="closed-badge">Closed</span>{/if}
    </h3>
    <p class="box-description">{excerpt}</p>
    <footer>
      <span class="idea-author">
        <img src={idea.author.avatarUrl} alt="" />
        <strong>{idea.author.login}</strong>
      </span>
      <span class="idea-meta-separator" aria-hidden="true">·</span>
      <time class="idea-date" datetime={idea.createdAt}>{date}</time>
      <span class="idea-meta-separator" aria-hidden="true">·</span>
      <span class="idea-comments">{idea.commentCount} {idea.commentCount === 1 ? 'comment' : 'comments'}</span>
      {#if !canVote}<span class="idea-meta-separator idea-signin-separator" aria-hidden="true">·</span><span class="idea-signin">Sign in to vote</span>{/if}
    </footer>
  </div>
  {#if idea.status === 'done' && idea.doneUrl}
    <a class="done-card-link" href={idea.doneUrl} target="_blank" rel="noreferrer" aria-label={`Open app for ${idea.title}`} title="Open app" on:click|stopPropagation>
      <span>Open app</span>
      <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">
        <path d="M7 17L17 7" />
        <path d="M9 7h8v8" />
      </svg>
    </a>
  {/if}
</div>
