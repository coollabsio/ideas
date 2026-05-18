<script lang="ts">
  import { QueryObserver, useQueryClient, type QueryObserverResult } from '@tanstack/svelte-query';
  import { readable } from 'svelte/store';
  import AuthSlot from '$lib/components/AuthSlot.svelte';
  import IdeaCard from '$lib/components/IdeaCard.svelte';
  import IdeaDetailsDialog from '$lib/components/IdeaDetailsDialog.svelte';
  import NewIdeaDialog from '$lib/components/NewIdeaDialog.svelte';
  import { ideaKeys, ideasQueryOptions, logout, meKeys, meQueryOptions, setUpvote, type Idea, type MeResponse } from '$lib/api';

  const queryClient = useQueryClient();
  const meQuery = queryResultStore<MeResponse>(meQueryOptions);
  const ideasQuery = queryResultStore<Idea[]>(ideasQueryOptions);

  let actionError = '';
  let dialogOpen = false;
  let selectedIdea: Idea | null = null;
  let busyIdea: string | null = null;

  $: ideas = sortIdeas($ideasQuery.data ?? []);
  $: me = $meQuery.data ?? { user: null, csrfToken: null };
  $: loading = $ideasQuery.isPending || $meQuery.isPending;
  $: queryError = [$ideasQuery.error, $meQuery.error]
    .filter(Boolean)
    .map((err) => (err instanceof Error ? err.message : 'Could not load app data.'))
    .join(' ');
  $: error = actionError || queryError;
  $: inProgressIdeas = ideas.filter((idea) => idea.status === 'inprogress');
  $: openIdeas = ideas.filter((idea) => idea.status === 'open');
  $: closedIdeas = ideas.filter((idea) => idea.status === 'closed');
  $: totalUpvotes = ideas.reduce((sum, idea) => sum + idea.upvoteCount, 0);
  $: if (selectedIdea) {
    const refreshed = ideas.find((idea) => idea.id === selectedIdea?.id);
    if (refreshed && refreshed !== selectedIdea) selectedIdea = refreshed;
  }

  function sortIdeas(next: Idea[]) {
    return [...next].sort((a, b) => b.upvoteCount - a.upvoteCount || b.createdAt.localeCompare(a.createdAt));
  }

  function queryResultStore<TData>(options: () => { queryKey: readonly unknown[]; queryFn: () => Promise<TData> }) {
    const observer = new QueryObserver<TData, Error>(queryClient, options());
    return readable<QueryObserverResult<TData, Error>>(observer.getCurrentResult(), (set) => {
      const unsubscribe = observer.subscribe(set);
      observer.updateResult();
      return () => {
        unsubscribe();
        observer.destroy();
      };
    });
  }

  function setIdeas(updater: (current: Idea[]) => Idea[]) {
    const current = queryClient.getQueryData<Idea[]>(ideaKeys.lists()) ?? [];
    queryClient.setQueryData<Idea[]>(ideaKeys.lists(), sortIdeas(updater(current)));
  }

  async function toggleUpvote(idea: Idea) {
    if (!me.user || !me.csrfToken) {
      window.location.href = '/api/auth/login';
      return;
    }
    busyIdea = idea.id;
    actionError = '';
    try {
      const updated = await setUpvote(idea.id, !idea.viewerHasUpvoted, me.csrfToken);
      replaceIdea(updated);
    } catch (err) {
      actionError = err instanceof Error ? err.message : 'Could not update upvote.';
    } finally {
      busyIdea = null;
    }
  }

  function ideaCreated(idea: Idea) {
    setIdeas((current) => [idea, ...current]);
  }

  function replaceIdea(idea: Idea) {
    setIdeas((current) => current.map((item) => (item.id === idea.id ? idea : item)));
    if (selectedIdea?.id === idea.id) selectedIdea = idea;
  }

  function deleteLocalIdea(idea: Idea) {
    setIdeas((current) => current.filter((item) => item.id !== idea.id));
    if (selectedIdea?.id === idea.id) selectedIdea = null;
  }

  async function signOut() {
    actionError = '';
    try {
      if (me.csrfToken) {
        await logout(me.csrfToken);
      }
      queryClient.setQueryData<MeResponse>(meKeys.current(), { user: null, csrfToken: null });
      await queryClient.invalidateQueries({ queryKey: ideaKeys.lists() });
    } catch (err) {
      actionError = err instanceof Error ? err.message : 'Could not sign out.';
    }
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
          <dd>{openIdeas.length + inProgressIdeas.length}</dd>
        </div>
        <div>
          <dt>Upvotes</dt>
          <dd>{totalUpvotes}</dd>
        </div>
      </dl>
    </section>

    {#if error}<div class="callout callout-warning"><strong>Notice ·</strong> {error}</div>{/if}

    {#if loading}
      <p class="empty">Loading ideas…</p>
    {:else if ideas.length === 0}
      <p class="empty">No ideas yet. Sign in with GitHub and submit the first one.</p>
    {:else}
      {#if inProgressIdeas.length > 0}
        <details class="idea-section inprogress-list" open>
          <summary class="list-head">
            <h2 id="inprogress-ideas-heading"><span>▸</span> In progress <small>({inProgressIdeas.length})</small></h2>
            <span>currently being worked on</span>
          </summary>
          <div class="ideas-list">
            {#each inProgressIdeas as idea (idea.id)}
              <IdeaCard {idea} canVote={Boolean(me.user)} busy={busyIdea === idea.id} onToggle={toggleUpvote} onOpen={(item) => (selectedIdea = item)} />
            {/each}
          </div>
        </details>
      {/if}

      <details class="idea-section all-ideas-list" open>
        <summary class="list-head">
          <h2><span>▸</span> All ideas <small>({openIdeas.length})</small></h2>
          <span>sorted by upvotes</span>
        </summary>
        {#if openIdeas.length > 0}
          <div class="ideas-list">
            {#each openIdeas as idea (idea.id)}
              <IdeaCard {idea} canVote={Boolean(me.user)} busy={busyIdea === idea.id} onToggle={toggleUpvote} onOpen={(item) => (selectedIdea = item)} />
            {/each}
          </div>
        {:else}
          <p class="empty">No open ideas right now.</p>
        {/if}
      </details>

      {#if closedIdeas.length > 0}
        <details class="closed-list">
          <summary><span>▸</span> Closed ideas <small>({closedIdeas.length})</small></summary>
          <div class="ideas-list">
            {#each closedIdeas as idea (idea.id)}
              <IdeaCard {idea} canVote={Boolean(me.user)} busy={busyIdea === idea.id} onToggle={toggleUpvote} onOpen={(item) => (selectedIdea = item)} />
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
<IdeaDetailsDialog idea={selectedIdea} csrfToken={me.csrfToken ?? null} onClose={() => (selectedIdea = null)} onUpdated={replaceIdea} onDeleted={deleteLocalIdea} />
