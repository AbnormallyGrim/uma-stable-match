<script lang="ts">
  import { onMount } from 'svelte';

  type Health = 'checking' | 'online' | 'unreachable';

  let health = $state<Health>('checking');

  onMount(async () => {
    try {
      const response = await fetch('/api/v1/health');
      health = response.ok ? 'online' : 'unreachable';
    } catch {
      health = 'unreachable';
    }
  });
</script>

<main>
  <h1>Uma Stable Match</h1>
  <p>Server: <span data-testid="health">{health}</span></p>
</main>
