<!--
  @component
  The page footer: tips for switching clubs, the logo, a link to report a problem, and the
  disclaimer and attributions. Keep the wording in step with the root README's Disclaimer section.
-->

<script lang="ts">
  import BrandLogo from './BrandLogo.svelte';

  const REPO = 'https://github.com/AbnormallyGrim/uma-stable-match';
  const KOFI = 'https://ko-fi.com/L5P6299MV1';
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
      <a href="{REPO}/issues" target="_blank" rel="noopener noreferrer">Report a problem</a>
      <a class="kofi" href={KOFI} target="_blank" rel="noopener noreferrer"
        ><span aria-hidden="true">🥕</span> Buy me a carrot</a
      >
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

  nav a {
    padding: 6px 14px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-weight: 600;
    text-decoration: none;
  }

  nav a:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  nav a.kofi {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  nav a.kofi:hover {
    background: color-mix(in srgb, var(--accent) 88%, var(--text));
    color: var(--on-accent);
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
