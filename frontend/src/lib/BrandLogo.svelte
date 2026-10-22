<!--
  @component
  The logo at `size` pixels, in the version for the current theme. Decorative, so it has no alt
  text; place it next to the project's name.
-->

<script lang="ts">
  import dark from '../assets/brand/logo-dark.webp';
  import light from '../assets/brand/logo-light.webp';

  let { size }: { size: number } = $props();
</script>

<span class="logo" style:--size="{size}px">
  <img class="only-light" src={light} alt="" width={size} height={size} />
  <img class="only-dark" src={dark} alt="" width={size} height={size} />
</span>

<style>
  .logo {
    display: inline-flex;
    flex: none;
  }

  img {
    width: var(--size);
    height: var(--size);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
</style>
