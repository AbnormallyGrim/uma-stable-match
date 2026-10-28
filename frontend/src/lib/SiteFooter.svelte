<!--
  @component
  The page footer: tips for switching clubs, the logo, a link to report a problem, and the
  disclaimer and attributions. Keep the wording in step with the root README's Disclaimer section.
-->

<script lang="ts">
  import carats from '../assets/carats/carats.webp';
  import BrandLogo from './BrandLogo.svelte';

  const REPO = 'https://github.com/AbnormallyGrim/uma-stable-match';
  const KOFI = 'https://ko-fi.com/L5P6299MV1';
  const FREE_CARATS = 'https://youtu.be/dQw4w9WgXcQ';

  let { today = new Date() }: { today?: Date } = $props();
  const open = (url: string) => window.open(url, '_blank', 'noopener,noreferrer');

  const aprilFools = $derived(today.getMonth() === 3 && today.getDate() === 1);
</script>

<footer class="site-footer">
  <details class="tips">
    <summary>Tips for switching clubs</summary>
    <ul>
      <li>
        <strong>14-day rule:</strong> you need at least 14 days in a club during a month to receive its
        monthly reward, so switch early in the month.
      </li>
      <li>
        <strong>24-hour cooldown:</strong> after leaving a club you can't join another for 24 hours. Players
        report that being removed by the leader avoids this.
      </li>
      <li>
        <strong>Month-start lock:</strong> from 10:00 to 14:59 UTC on the 1st, you can't leave, apply
        to or invite to clubs.
      </li>
      <li>
        <strong>Check activity first:</strong> review members' activity on uma.moe before applying.
      </li>
    </ul>
    <p>These rules can change, so check in-game if something doesn't match.</p>
  </details>

  <div class="meta">
    <div class="brand">
      <BrandLogo size={32} />
      <span>Uma Stable Match</span>
    </div>

    <nav aria-label="Project links">
      <button type="button" onclick={() => open(`${REPO}/issues`)}>Report a problem</button>
      <button type="button" class="kofi" onclick={() => open(KOFI)}
        ><span aria-hidden="true">🥕</span> Buy me a carrot</button
      >
      {#if aprilFools}
        <button type="button" class="carats" onclick={() => open(FREE_CARATS)}
          ><img src={carats} alt="" width="20" height="20" /> Free carats</button
        >
      {/if}
    </nav>
    <p class="support">Optional tips help cover hosting. Every feature stays free.</p>

    <div class="notes">
      <p>
        Uma Stable Match is an <strong>unofficial, non-commercial fan tool</strong>. It isn't
        affiliated with, endorsed by or sponsored by uma.moe or Cygames, Inc.
      </p>
      <p>
        Club and trainer data comes from
        <a href="https://uma.moe" target="_blank" rel="noopener noreferrer">uma.moe</a> and may be delayed
        or incomplete. Recommendations are estimates to help you decide, not judgements of any player
        or club.
      </p>
      <p>
        <em>Umamusume: Pretty Derby</em> and its names, characters, artwork and trademarks belong to
        Cygames, Inc. The club tier icons are © Cygames, Inc. and are used non-commercially in line
        with Cygames'
        <a href="https://umamusume.com/fan-createdguide" target="_blank" rel="noopener noreferrer"
          >Fan-Created Content Guidelines</a
        >.
      </p>
    </div>
  </div>
</footer>

<style>
  .site-footer {
    display: grid;
    gap: 32px;
    max-width: var(--page-max);
    margin: 0 auto;
    padding: 40px var(--gutter) 48px;
    border-top: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.8125rem;
  }

  .tips {
    width: 100%;
    max-width: 760px;
    justify-self: center;
    padding: 12px 18px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .tips[open] {
    border-color: color-mix(in srgb, var(--accent) 35%, transparent);
  }

  summary {
    color: var(--text);
    font-size: 0.9375rem;
    font-weight: 600;
    cursor: pointer;
  }

  ul {
    margin: 10px 0 0;
    padding-left: 20px;
  }

  li + li {
    margin-top: 4px;
  }

  .tips p {
    margin: 10px 0 0;
  }

  .meta {
    display: grid;
    gap: 18px;
    justify-items: center;
    text-align: center;
  }

  .brand {
    display: flex;
    gap: 10px;
    align-items: center;
    color: var(--text);
    font-size: 1rem;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: center;
  }

  nav button {
    font: inherit;
    cursor: pointer;
    padding: 6px 14px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-weight: 600;
    text-decoration: none;
  }

  nav button:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  nav button.kofi {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  nav button.kofi:hover {
    background: color-mix(in srgb, var(--accent) 88%, var(--text));
    color: var(--on-accent);
  }

  nav button.carats {
    --rainbow: linear-gradient(
      90deg,
      #ff3b3b,
      #ff9f1c,
      #ffe600,
      #2ecc71,
      #19b5fe,
      #7b61ff,
      #ff3bd4,
      #ff3b3b
    );

    position: relative;
    isolation: isolate;
    display: inline-flex;
    gap: 6px;
    align-items: center;
    border: 2px solid transparent;
    background:
      linear-gradient(var(--card), var(--card)) padding-box,
      var(--rainbow) border-box 0 0 / 200% 100%;
    animation: rainbow-slide 4s linear infinite;
  }

  nav button.carats::before {
    content: '';
    position: absolute;
    inset: -2px;
    z-index: -1;
    border-radius: inherit;
    background: var(--rainbow) 0 0 / 200% 100%;
    filter: blur(10px);
    opacity: 0.6;
    animation: rainbow-slide 4s linear infinite;
  }

  nav button.carats:hover {
    color: var(--text);
  }

  nav button.carats img {
    width: 20px;
    height: 20px;
  }

  @keyframes rainbow-slide {
    to {
      background-position: 200% 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    nav button.carats,
    nav button.carats::before {
      animation: none;
    }
  }

  .support {
    margin: -8px 0 0;
  }

  .notes {
    max-width: 760px;
  }

  .notes p {
    margin: 0 0 8px;
  }

  strong {
    color: var(--text-2);
  }
</style>
