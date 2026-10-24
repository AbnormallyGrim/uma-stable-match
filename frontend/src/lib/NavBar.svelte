<!--
  @component
  The top bar: the brand, linking back to the website, and the theme switch. It matches the
  website's navbar and stays at the top while scrolling.
-->

<script lang="ts">
  import BrandLogo from './BrandLogo.svelte';
  import ThemeToggle from './ThemeToggle.svelte';

  const WEBSITE = 'https://umastable.com';
</script>

<nav class="navbar" aria-label="Main">
  <div class="inner">
    <a class="brand" href={WEBSITE}>
      <BrandLogo size={40} />
      <span class="title">Uma Stable Match</span>
    </a>
    <ThemeToggle />
  </div>
</nav>

<style>
  .navbar {
    position: sticky;
    z-index: 100;
    top: 0;
    background: color-mix(in srgb, var(--bg) 85%, transparent);
    border-bottom: 1px solid var(--border);
    backdrop-filter: blur(12px);
  }

  .inner {
    display: flex;
    gap: 16px;
    align-items: center;
    justify-content: space-between;
    max-width: var(--page-max);
    margin: 0 auto;
    padding: 10px var(--gutter);
  }

  .brand {
    display: flex;
    gap: 12px;
    align-items: center;
    min-width: 0;
    color: var(--text);
    text-decoration: none;
  }

  .title {
    overflow: hidden;
    font-size: 1.15rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 400px) {
    .title {
      font-size: 1rem;
    }
  }
</style>
