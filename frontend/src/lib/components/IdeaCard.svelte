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

<div class="idea-card coolbox group" class:closed={idea.closed} role="button" tabindex="0" on:click={() => onOpen(idea)} on:keydown={handleKeydown} aria-label={`Open idea: ${idea.title}`}>
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
      {#if idea.closed}<span class="closed-badge">Closed</span>{/if}
    </h3>
    <p class="box-description">{excerpt}</p>
    <footer>
      <img src={idea.author.avatarUrl} alt="" />
      <strong>{idea.author.login}</strong>
      <span aria-hidden="true">·</span>
      <time datetime={idea.createdAt}>{date}</time>
      {#if !canVote}<span aria-hidden="true">·</span><span>Sign in to vote</span>{/if}
    </footer>
  </div>
</div>
