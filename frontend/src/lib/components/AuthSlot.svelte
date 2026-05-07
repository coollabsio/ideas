<script lang="ts">
  import type { MeResponse } from '$lib/api';

  export let me: MeResponse = { user: null, csrfToken: null };
  export let onNewIdea: () => void;
  export let onLogout: () => void;
</script>

<div class="auth-slot">
  {#if me.user}
    <button class="button button-highlighted" type="button" on:click={onNewIdea}>+ New idea</button>
    <div class="user-pill">
      <img src={me.user.avatarUrl} alt="" />
      <span>{me.user.login}</span>
    </div>
    <button class="button button-ghost icon-button" type="button" on:click={onLogout} aria-label="Sign out" title="Sign out">↗</button>
  {:else}
    <a class="button" href="/api/auth/login">Sign in</a>
  {/if}
</div>
