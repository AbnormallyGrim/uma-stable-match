<script lang="ts">
  import type { Club } from './api';
  import {
    formatPercent,
    formatPerDay,
    formatPlace,
    formatScore,
    joinStyleLabel,
    tierIndex,
  } from './format';
  import TierBadge from './TierBadge.svelte';

  type SortKey =
    | 'pick'
    | 'tier'
    | 'name'
    | 'rank'
    | 'lastRank'
    | 'spots'
    | 'join'
    | 'median'
    | 'place'
    | 'balance'
    | 'passengers'
    | 'top3'
    | 'score';
  type Direction = 'ascending' | 'descending';

  interface Column {
    key: SortKey;
    label: string;
    value: (row: Row) => number | string | null;
    first: Direction;
    balanceOnly?: boolean;
  }

  interface Row {
    club: Club;
    pick: number;
  }

  interface Props {
    clubs: Club[];
    balanceOn: boolean;
    includeClosed: boolean;
  }

  let { clubs, balanceOn, includeClosed }: Props = $props();

  const columns: Column[] = [
    { key: 'pick', label: '#', value: (r) => r.pick, first: 'ascending' },
    {
      key: 'tier',
      label: 'Tier',
      value: (r) => (tierIndex(r.club.tier) < 0 ? null : tierIndex(r.club.tier)),
      first: 'descending',
    },
    { key: 'name', label: 'Club', value: (r) => r.club.name.toLowerCase(), first: 'ascending' },
    { key: 'rank', label: 'Rank', value: (r) => r.club.monthly_rank, first: 'ascending' },
    {
      key: 'lastRank',
      label: 'Last month',
      value: (r) => r.club.last_month_rank,
      first: 'ascending',
    },
    { key: 'spots', label: 'Open spots', value: (r) => r.club.open_spots, first: 'descending' },
    {
      key: 'join',
      label: 'Join style',
      value: (r) => joinStyleLabel(r.club.join_style),
      first: 'ascending',
    },
    {
      key: 'median',
      label: 'Median/day',
      value: (r) => r.club.median_per_day,
      first: 'descending',
    },
    {
      key: 'place',
      label: "You'd be",
      value: (r) => r.club.your_place / r.club.club_size,
      first: 'ascending',
    },
    {
      key: 'balance',
      label: 'Balance',
      value: (r) => r.club.balance?.score ?? null,
      first: 'descending',
      balanceOnly: true,
    },
    {
      key: 'passengers',
      label: 'Passengers',
      value: (r) => r.club.balance?.passengers ?? null,
      first: 'ascending',
      balanceOnly: true,
    },
    {
      key: 'top3',
      label: 'Top 3 share',
      value: (r) => r.club.balance?.top3_share ?? null,
      first: 'ascending',
      balanceOnly: true,
    },
    {
      key: 'score',
      label: 'Fit',
      value: (r) => (balanceOn ? r.club.score : r.club.fit),
      first: 'ascending',
    },
  ];

  let search = $state('');
  let hideClosed = $state(true);
  let sortKey = $state<SortKey>('pick');
  let direction = $state<Direction>('ascending');

  const visibleColumns = $derived(
    columns
      .filter((c) => balanceOn || !c.balanceOnly)
      .map((c) => (c.key === 'score' && balanceOn ? { ...c, label: 'Score' } : c)),
  );
  const rows = $derived(clubs.map((club, i) => ({ club, pick: i + 1 })));
  const filtered = $derived.by(() => {
    const term = search.trim().toLowerCase();
    return rows.filter(
      (r) =>
        (term === '' || r.club.name.toLowerCase().includes(term)) &&
        !(includeClosed && hideClosed && r.club.join_style === 'closed'),
    );
  });
  const sorted = $derived.by(() => {
    const column = columns.find((c) => c.key === sortKey) ?? columns[0]!;
    const sign = direction === 'ascending' ? 1 : -1;
    return [...filtered].sort((a, b) => {
      const x = column.value(a);
      const y = column.value(b);
      if (x === y) return a.pick - b.pick;
      if (x === null) return 1;
      if (y === null) return -1;
      const order =
        typeof x === 'string' && typeof y === 'string' ? x.localeCompare(y) : x < y ? -1 : 1;
      return order * sign;
    });
  });

  function sortBy(column: Column) {
    if (sortKey === column.key) {
      direction = direction === 'ascending' ? 'descending' : 'ascending';
    } else {
      sortKey = column.key;
      direction = column.first;
    }
  }

  function spots(club: Club): string {
    return club.open_spots === null ? '–' : String(club.open_spots);
  }
</script>

<div class="toolbar">
  <label>
    <span class="visually-hidden">Search clubs by name</span>
    <input type="search" placeholder="Search clubs" bind:value={search} />
  </label>
  {#if includeClosed}
    <label class="check">
      <input type="checkbox" bind:checked={hideClosed} />
      Hide closed clubs
    </label>
  {/if}
  <span class="count" aria-live="polite">
    Showing {sorted.length} of {clubs.length}
    {clubs.length === 1 ? 'club' : 'clubs'}
  </span>
</div>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="scroll" role="region" aria-label="All results" tabindex="0">
  <table>
    <thead>
      <tr>
        {#each visibleColumns as column (column.key)}
          <th
            scope="col"
            aria-sort={sortKey === column.key ? direction : undefined}
            class:name={column.key === 'name'}
          >
            <button type="button" onclick={() => sortBy(column)}>
              {column.label}
              <span class="arrow" aria-hidden="true"
                >{sortKey === column.key ? (direction === 'ascending' ? '▲' : '▼') : ''}</span
              >
            </button>
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each sorted as { club, pick } (club.circle_id)}
        <tr>
          <td>{pick}</td>
          <td><TierBadge tier={club.tier} size="small" /></td>
          <td class="name">
            <a href={club.url} target="_blank" rel="noopener noreferrer">{club.name}</a>
          </td>
          <td>{club.monthly_rank ?? '–'}</td>
          <td>{club.last_month_rank ?? '–'}</td>
          <td>{spots(club)}</td>
          <td>{joinStyleLabel(club.join_style) ?? '–'}</td>
          <td>{formatPerDay(club.median_per_day)}</td>
          <td>{formatPlace(club.your_place, club.club_size)}</td>
          {#if balanceOn}
            <td>{club.balance ? `${club.balance.score}/100` : '–'}</td>
            <td>
              {club.balance ? formatPlace(club.balance.passengers, club.balance.counted) : '–'}
            </td>
            <td>{club.balance ? formatPercent(club.balance.top3_share) : '–'}</td>
          {/if}
          <td>{formatScore(balanceOn ? club.score : club.fit)}</td>
        </tr>
      {:else}
        <tr>
          <td class="empty" colspan={visibleColumns.length}>No clubs match.</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    margin-bottom: 10px;
    color: var(--text-2);
    font-size: 0.8125rem;
  }

  input[type='search'] {
    min-width: 200px;
    padding: 6px 10px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
  }

  .count {
    margin-left: auto;
  }

  .scroll {
    overflow-x: auto;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8125rem;
    font-variant-numeric: tabular-nums;
  }

  th,
  td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    text-align: right;
    white-space: nowrap;
  }

  th.name,
  td.name {
    text-align: left;
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }

  tbody tr:hover {
    background: var(--surface);
  }

  th {
    position: sticky;
    top: 0;
    background: var(--card);
  }

  th button {
    padding: 0;
    background: none;
    border: 0;
    color: var(--text-2);
    font: inherit;
    font-size: 0.6875rem;
    font-weight: 600;
    cursor: pointer;
  }

  th button:hover,
  th button:focus-visible {
    color: var(--text);
  }

  .arrow {
    display: inline-block;
    width: 1em;
  }

  td a {
    font-weight: 600;
    text-decoration: none;
  }

  td a:hover,
  td a:focus-visible {
    text-decoration: underline;
  }

  .empty {
    padding: 16px;
    color: var(--muted);
    text-align: center;
  }
</style>
