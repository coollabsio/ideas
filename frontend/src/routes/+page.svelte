<script lang="ts">
  import { onMount } from 'svelte';
  import AuthSlot from '$lib/components/AuthSlot.svelte';
  import IdeaCard from '$lib/components/IdeaCard.svelte';
  import NewIdeaDialog from '$lib/components/NewIdeaDialog.svelte';
  import { fetchIdeas, fetchMe, logout, setUpvote, type Idea, type MeResponse } from '$lib/api';

  let ideas: Idea[] = [];
  let me: MeResponse = { user: null, csrfToken: null };
  let loading = true;
  let error = '';
  let dialogOpen = false;
  let busyIdea: string | null = null;

  $: openIdeas = ideas.filter((idea) => !idea.closed);
  $: closedIdeas = ideas.filter((idea) => idea.closed);
  $: totalUpvotes = ideas.reduce((sum, idea) => sum + idea.upvoteCount, 0);

  onMount(async () => {
    await Promise.all([loadMe(), loadIdeas()]);
    loading = false;
  });

  async function loadMe() {
    try {
      me = await fetchMe();
    } catch (err) {
      console.error(err);
    }
  }

  async function loadIdeas() {
    try {
      ideas = await fetchIdeas();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not load ideas.';
    }
  }

  function sortIdeas(next: Idea[]) {
    return [...next].sort((a, b) => b.upvoteCount - a.upvoteCount || b.createdAt.localeCompare(a.createdAt));
  }

  async function toggleUpvote(idea: Idea) {
    if (!me.user || !me.csrfToken) {
      window.location.href = '/api/auth/login';
      return;
    }
    busyIdea = idea.id;
    try {
      const updated = await setUpvote(idea.id, !idea.viewerHasUpvoted, me.csrfToken);
      ideas = sortIdeas(ideas.map((item) => (item.id === updated.id ? updated : item)));
    } catch (err) {
      error = err instanceof Error ? err.message : 'Could not update upvote.';
    } finally {
      busyIdea = null;
    }
  }

  function ideaCreated(idea: Idea) {
    ideas = sortIdeas([idea, ...ideas]);
  }

  async function signOut() {
    if (me.csrfToken) await logout(me.csrfToken);
    me = { user: null, csrfToken: null };
    await loadIdeas();
  }
</script>

<svelte:head>
  <meta property="og:title" content="coolLabs · Ideas" />
  <meta property="og:description" content="Submit and upvote ideas for cool open-source apps." />
  <meta property="og:image" content="/og-image.png" />
</svelte:head>

<div class="shell">
  <header class="topbar">
    <a href="/" class="brand"><span></span> Coollabs <b>/</b> Ideas</a>
    <AuthSlot {me} onNewIdea={() => (dialogOpen = true)} onLogout={signOut} />
  </header>

  <main>
    <section class="hero">
      <h1>Ideas worth building.</h1>
      <dl class="stats">
        <div>
          <dt>Ideas</dt>
          <dd>{openIdeas.length}</dd>
        </div>
        <div>
          <dt>Upvotes</dt>
          <dd>{totalUpvotes}</dd>
        </div>
      </dl>
    </section>

    {#if error}<div class="callout callout-warning"><strong>Notice ·</strong> {error}</div>{/if}

    <div class="list-head">
      <h2><span>▸</span> All ideas</h2>
      <span>sorted by upvotes</span>
    </div>

    {#if loading}
      <p class="empty">Loading ideas…</p>
    {:else if ideas.length === 0}
      <p class="empty">No ideas yet. Sign in with GitHub and submit the first one.</p>
    {:else}
      <div class="ideas-list">
        {#each openIdeas as idea (idea.id)}
          <IdeaCard {idea} canVote={Boolean(me.user)} busy={busyIdea === idea.id} onToggle={toggleUpvote} />
        {/each}
      </div>

      {#if closedIdeas.length > 0}
        <details class="closed-list">
          <summary><span>▸</span> Closed ideas <small>({closedIdeas.length})</small></summary>
          <div class="ideas-list">
            {#each closedIdeas as idea (idea.id)}
              <IdeaCard {idea} canVote={Boolean(me.user)} busy={busyIdea === idea.id} onToggle={toggleUpvote} />
            {/each}
          </div>
        </details>
      {/if}
    {/if}
  </main>

  <footer class="footer">
    <span>Built with <strong>Rust</strong> · Powered by <a href="https://coolify.io?ref=coollabsideas">Coolify</a></span>
  </footer>
</div>

<NewIdeaDialog open={dialogOpen} csrfToken={me.csrfToken ?? null} onClose={() => (dialogOpen = false)} onCreated={ideaCreated} />
