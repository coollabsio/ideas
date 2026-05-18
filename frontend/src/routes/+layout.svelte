<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { QueryClient, setQueryClientContext } from '@tanstack/svelte-query';
  import '../app.css';

  const queryClient = new QueryClient();

  setQueryClientContext(queryClient);

  onMount(() => {
    queryClient.mount();
  });

  onDestroy(() => {
    queryClient.unmount();
  });
</script>

<slot />
