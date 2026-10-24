<!--
  @component
  A collapsible "How to use" section: finding the trainer ID or daily pace, the options, and picking
  a club. Closed by default. Keep the labels it quotes in step with the search form and results.
-->

<script lang="ts">
  const RANKINGS = 'https://uma.moe/rankings';
</script>

<details class="how-to" data-tour="how-to">
  <summary>How to use</summary>
  <ol>
    <li>
      <strong>Find your trainer ID.</strong> In the game, open
      <strong>Menu → Profile Trainer Card</strong>. Your trainer ID is the 9 to 12 digit number on
      your profile.
    </li>
    <li>
      <strong>Search with it.</strong> Enter it under <strong>Trainer ID</strong> and select
      <strong>Find clubs</strong>. Tick <strong>Remember my trainer ID on this device</strong> to have
      it filled in next time.
    </li>
    <li>
      <strong>Or search by your pace.</strong> Switch to <strong>Daily pace</strong> and enter how
      many fans you gain a day, in millions: <strong>7.6</strong> means 7.6 million a day. To find
      it, search for your trainer name or ID in
      <a href={RANKINGS} target="_blank" rel="noopener">uma.moe's rankings</a>
      and use your average daily fan gain.
    </li>
    <li>
      <strong>Fine-tune, if you like.</strong> <strong>Balance scoring</strong> ranks clubs where a
      few members carry everyone lower. <strong>Include full clubs</strong> adds clubs with no open
      spot to the table, to compare against. Pick one <strong>Join style</strong>, such as
      <strong>Closed</strong> for clubs that aren't recruiting right now, or one
      <strong>Playstyle</strong>, to see only those clubs. Pick it again, or choose
      <strong>Any playstyle</strong>, to go back. Either leaves fewer clubs to choose from, so your
      best matches may be further from your pace. <strong>Has Discord</strong> keeps clubs whose description
      mentions Discord, as a best guess.
    </li>
    <li>
      <strong>Pick a club.</strong> Start with the <strong>Top picks</strong>: the best fits are
      clubs where you'd sit around the middle of the members. Use <strong>Top results</strong> to search
      and sort more. Select a club's name to open its uma.moe page and contact the leader.
    </li>
  </ol>
  <p class="note">
    Works with the global (English) version of the game. If your trainer ID isn't found, search by
    your daily pace instead. Full clubs and your own club aren't in the results. Search by trainer
    ID to see how your club compares.
  </p>
</details>

<style>
  .how-to {
    margin-top: 12px;
    padding: 0 16px;
    background: color-mix(in srgb, var(--card) 55%, transparent);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    backdrop-filter: blur(2px);
  }

  summary {
    padding: 12px 0;
    font-weight: 600;
    cursor: pointer;
  }

  ol {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0 0 0 20px;
  }

  li {
    line-height: 1.5;
  }

  a {
    color: var(--accent);
  }

  .note {
    margin: 12px 0 16px;
    color: var(--muted);
    font-size: 0.875rem;
  }
</style>
