<!--
  @component
  A trainer's leader character icon in a circle. Without an image for their outfit, it shows the
  first letter of their name.
-->

<script lang="ts">
  import { dressIconUrl } from './ads';

  interface Props {
    /** The trainer's name, for the letter shown without an image. */
    name: string;
    /** The outfit ID of their leader character, if known. */
    dressId: number | null;
    /** The circle's width and height in pixels. */
    size?: number;
  }

  let { name, dressId, size = 48 }: Props = $props();

  const icon = $derived(dressId === null ? null : dressIconUrl(dressId));
</script>

<span class="icon" style:--size="{size}px" aria-hidden="true">
  {name.slice(0, 1)}
  {#if icon}
    <img src={icon} alt="" />
  {/if}
</span>

<style>
  .icon {
    position: relative;
    display: grid;
    flex: none;
    place-items: center;
    overflow: hidden;
    width: var(--size);
    height: var(--size);
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 50%;
    color: var(--muted);
    font-weight: 700;
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
