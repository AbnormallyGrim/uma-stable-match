<!--
  @component
  The page footer: tips for switching clubs, the privacy note, the disclaimer and attributions,
  and project links. Keep the wording in step with the root README's Disclaimer section.
-->

<script lang="ts">
  const REPO = 'https://github.com/AbnormallyGrim/uma-stable-match';
</script>

<footer class="site-footer">
  <details>
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

  <details>
    <summary>Privacy</summary>
    <ul>
      <li>
        When you search by trainer ID, the server asks uma.moe for your club's fan data to work out
        your pace. Your pace is kept in memory for a few minutes, so repeat searches are fast, and
        then forgotten. Trainer IDs aren't logged or written to disk.
      </li>
      <li>
        To prevent abuse, the server counts searches per IP address. The counts are kept in memory
        only, and IP addresses aren't logged.
      </li>
      <li>
        If you tick "Remember my trainer ID", it's saved in this browser only. Your theme choice is
        saved the same way. You can clear both by clearing this site's data in your browser.
      </li>
      <li>There are no accounts, cookies, ads or trackers.</li>
    </ul>
  </details>

  <div class="notes">
    <p>
      Uma Stable Match is an <strong>unofficial, non-commercial fan tool</strong>. It isn't
      affiliated with, endorsed by or sponsored by uma.moe or Cygames, Inc.
    </p>
    <p>
      Club and trainer data comes from
      <a href="https://uma.moe" target="_blank" rel="noopener noreferrer">uma.moe</a> and may be delayed
      or incomplete. Recommendations are estimates to help you decide, not judgements of any player or
      club.
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

  <nav aria-label="Project links">
    <a href={REPO} target="_blank" rel="noopener noreferrer">Source code</a>
    <a href="{REPO}/blob/main/LICENSE" target="_blank" rel="noopener noreferrer">MIT licence</a>
    <a href="{REPO}/issues" target="_blank" rel="noopener noreferrer">Report a problem</a>
  </nav>
</footer>

<style>
  .site-footer {
    display: grid;
    gap: 12px;
    max-width: 1100px;
    margin: 0 auto;
    padding: 24px 16px 48px;
    border-top: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.8125rem;
  }

  details {
    padding: 8px 12px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  summary {
    color: var(--text);
    font-weight: 600;
    cursor: pointer;
  }

  ul {
    margin: 8px 0 0;
    padding-left: 20px;
  }

  li + li {
    margin-top: 4px;
  }

  details p {
    margin: 8px 0 0;
  }

  .notes p {
    margin: 0 0 6px;
  }

  strong {
    color: var(--text-2);
  }

  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
  }
</style>
