<!--
  @component
  The club page chart: each member's daily gain or fans this month as a thin muted line, the club
  median in bold, the trainer's line in the accent colour and the chosen member's line in the
  highlight colour. A crosshair shows every line's value for a day, on hover or with the arrow keys,
  which move it like a slider over the days. In the current month, the day still under way is left
  out.
-->

<script lang="ts">
  import type { ClubDetails } from './api';
  import {
    lastDay,
    medianSeries,
    tickLabel,
    ticks,
    toSeries,
    type ChartMode,
    type Series,
  } from './chart';
  import { formatMillions, formatPace, gameMonth } from './format';
  import { t } from './i18n';

  interface Props {
    details: Pick<ClubDetails, 'members' | 'trainer' | 'month'>;
    /** The index in `details.members` of the member whose line stands out, if any. */
    highlight?: number | null;
    /** Replaceable in tests. */
    now?: Date;
  }

  let { details, highlight = null, now = new Date() }: Props = $props();

  let mode = $state<ChartMode>('daily');
  let width = $state(0);
  let hover = $state<number | null>(null);

  const HEIGHT = 260;
  const PAD = { top: 12, right: 16, bottom: 28, left: 48 };

  const members = $derived(details.members.map((m) => toSeries(m.daily_fans, mode)));
  const yours = $derived(
    details.trainer?.daily_fans ? toSeries(details.trainer.daily_fans, mode) : null,
  );
  const days = $derived(
    Math.max(0, ...details.members.map((m) => m.daily_fans.length), yours?.length ?? 0),
  );
  const median = $derived(medianSeries(members, days));
  const chosen = $derived(
    highlight === null || !members[highlight]
      ? null
      : {
          series: members[highlight]!,
          name: details.members[highlight]!.name ?? t('clubPage.unknown'),
        },
  );
  // In the current game month the latest day is still under way, so it would show as a sharp drop.
  const today = $derived(details.month === null || details.month === gameMonth(now.toISOString()));
  const last = $derived(lastDay(yours ? [...members, yours] : members) - (today ? 1 : 0));
  const first = $derived(mode === 'daily' ? 1 : 0);
  const yTicks = $derived(
    ticks(
      Math.max(
        0,
        ...[...members, median, yours ?? []].flatMap((s) =>
          s.slice(0, last + 1).filter((v): v is number => v !== null),
        ),
      ),
    ),
  );

  const plotWidth = $derived(Math.max(1, (width || 640) - PAD.left - PAD.right));
  const plotHeight = HEIGHT - PAD.top - PAD.bottom;
  const x = (day: number) => PAD.left + ((day - first) / Math.max(1, last - first)) * plotWidth;
  const y = (value: number) =>
    PAD.top + plotHeight - (value / yTicks[yTicks.length - 1]!) * plotHeight;

  function path(series: Series): string {
    return series
      .slice(0, last + 1)
      .flatMap((v, day) => (v === null ? [] : [`${x(day).toFixed(1)},${y(v).toFixed(1)}`]))
      .map((point, i) => `${i === 0 ? 'M' : 'L'}${point}`)
      .join('');
  }

  // Every fifth day, as the game shows them: day 1 is the first.
  const xTicks = $derived(
    Array.from({ length: last + 1 }, (_, day) => day).filter(
      (day) => day >= first && ((day + 1) % 5 === 0 || day === first),
    ),
  );

  // Where a line's last value on the chart is, for its end dot.
  function end(series: Series | null) {
    if (!series) return null;
    const day = lastDay([series.slice(0, last + 1)]);
    return day < 0 ? null : { day, value: series[day]! };
  }

  const yoursEnd = $derived(end(yours));
  const chosenEnd = $derived(end(chosen?.series ?? null));

  const format = (value: number) => (mode === 'daily' ? formatPace(value) : formatMillions(value));

  function valuesOn(day: number) {
    const values = members.map((s) => s[day]).filter((v): v is number => v != null);
    return {
      day,
      chosen: chosen?.series[day] ?? null,
      yours: yours?.[day] ?? null,
      median: median[day] ?? null,
      low: values.length ? Math.min(...values) : null,
      high: values.length ? Math.max(...values) : null,
    };
  }

  const readout = $derived(hover === null ? null : valuesOn(hover));

  // What a screen reader announces for the day the arrow keys are on: the tooltip's lines.
  const spoken = $derived.by(() => {
    const values = valuesOn(hover ?? last);
    const parts = [t('chart.day', { n: values.day + 1 })];
    if (chosen && values.chosen !== null) parts.push(`${chosen.name} ${format(values.chosen)}`);
    if (values.yours !== null) parts.push(`${t('chart.you')} ${format(values.yours)}`);
    if (values.median !== null) parts.push(`${t('chart.median')} ${format(values.median)}`);
    if (values.low !== null && values.high !== null) {
      const range = t('chart.range', { low: format(values.low), high: format(values.high) });
      parts.push(`${t('chart.members')} ${range}`);
    }
    return parts.join(', ');
  });

  const clamp = (day: number) => Math.min(last, Math.max(first, day));

  function onpointermove(event: PointerEvent) {
    const box = (event.currentTarget as SVGElement).getBoundingClientRect();
    const share = (event.clientX - box.left - PAD.left) / plotWidth;
    hover = clamp(Math.round(first + share * (last - first)));
  }

  function onkeydown(event: KeyboardEvent) {
    const moves: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1 };
    if (event.key in moves) {
      hover = clamp((hover ?? last) + moves[event.key]!);
    } else if (event.key === 'Home') {
      hover = first;
    } else if (event.key === 'End') {
      hover = last;
    } else {
      return;
    }
    event.preventDefault();
  }
</script>

<section class="chart" aria-labelledby="chart-heading">
  <div class="top">
    <h2 id="chart-heading">{t('chart.heading')}</h2>
    <fieldset class="segmented small">
      <legend class="visually-hidden">{t('chart.show')}</legend>
      <label>
        <input type="radio" name="chart-mode" value="daily" bind:group={mode} />
        {t('chart.daily')}
      </label>
      <label>
        <input type="radio" name="chart-mode" value="month" bind:group={mode} />
        {t('chart.month')}
      </label>
    </fieldset>
  </div>

  {#if last <= first}
    <p class="empty">{t('chart.empty')}</p>
  {:else}
    <ul class="legend">
      <li><span class="key members" aria-hidden="true"></span>{t('chart.members')}</li>
      <li><span class="key median" aria-hidden="true"></span>{t('chart.median')}</li>
      {#if yours}
        <li><span class="key yours" aria-hidden="true"></span>{t('chart.you')}</li>
      {/if}
      {#if chosen}
        <li><span class="key chosen" aria-hidden="true"></span>{chosen.name}</li>
      {/if}
    </ul>

    <div class="plot" bind:clientWidth={width}>
      <svg
        width={width || 640}
        height={HEIGHT}
        role="slider"
        tabindex="0"
        aria-label={t(mode === 'daily' ? 'chart.labelDaily' : 'chart.labelMonth')}
        aria-valuemin={first + 1}
        aria-valuemax={last + 1}
        aria-valuenow={(hover ?? last) + 1}
        aria-valuetext={spoken}
        {onpointermove}
        onpointerleave={() => (hover = null)}
        {onkeydown}
        onblur={() => (hover = null)}
      >
        <g class="grid">
          {#each yTicks as tick (tick)}
            <line x1={PAD.left} x2={PAD.left + plotWidth} y1={y(tick)} y2={y(tick)} />
            <text x={PAD.left - 8} y={y(tick)} dy="0.32em" text-anchor="end">{tickLabel(tick)}</text
            >
          {/each}
          {#each xTicks as day (day)}
            <text x={x(day)} y={HEIGHT - 8} text-anchor="middle">{day + 1}</text>
          {/each}
        </g>
        {#each members as series, i (i)}
          <path class="member" d={path(series)} />
        {/each}
        <path class="median" d={path(median)} />
        {#if yours}
          <path class="yours" d={path(yours)} />
          {#if yoursEnd}
            <circle class="end" cx={x(yoursEnd.day)} cy={y(yoursEnd.value)} r="4" />
          {/if}
        {/if}
        {#if chosen}
          <path class="chosen" d={path(chosen.series)} />
          {#if chosenEnd}
            <circle class="end chosen" cx={x(chosenEnd.day)} cy={y(chosenEnd.value)} r="4" />
          {/if}
        {/if}
        {#if readout}
          <line
            class="crosshair"
            x1={x(readout.day)}
            x2={x(readout.day)}
            y1={PAD.top}
            y2={PAD.top + plotHeight}
          />
        {/if}
      </svg>

      {#if readout}
        <div
          class="tooltip"
          aria-hidden="true"
          class:left={x(readout.day) > (width || 640) / 2}
          style:left="{x(readout.day)}px"
        >
          <p class="day">{t('chart.day', { n: readout.day + 1 })}</p>
          {#if chosen && readout.chosen !== null}
            <p><span class="key chosen"></span><b>{format(readout.chosen)}</b> {chosen.name}</p>
          {/if}
          {#if readout.yours !== null}
            <p><span class="key yours"></span><b>{format(readout.yours)}</b> {t('chart.you')}</p>
          {/if}
          {#if readout.median !== null}
            <p>
              <span class="key median"></span><b>{format(readout.median)}</b>
              {t('chart.median')}
            </p>
          {/if}
          {#if readout.low !== null && readout.high !== null}
            <p>
              <span class="key members"></span><b
                >{t('chart.range', { low: format(readout.low), high: format(readout.high) })}</b
              >
              {t('chart.members')}
            </p>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .chart {
    margin-top: 16px;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 16px;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  h2 {
    margin: 0;
    font-size: 1rem;
  }

  .empty {
    margin: 0;
    color: var(--muted);
    font-size: 0.875rem;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
    margin: 0 0 6px;
    padding: 0;
    color: var(--text-2);
    font-size: 0.8125rem;
    list-style: none;
  }

  .legend li {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .key {
    display: inline-block;
    flex: none;
    width: 14px;
    height: 0;
    border-top: 2px solid;
    vertical-align: middle;
  }

  .key.members {
    border-top-width: 1px;
    border-color: color-mix(in srgb, var(--muted) 60%, transparent);
  }

  .key.median {
    border-color: var(--text);
  }

  .key.yours {
    border-color: var(--accent);
  }

  .key.chosen {
    border-color: var(--highlight);
  }

  .plot {
    position: relative;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  svg {
    display: block;
    max-width: 100%;
    touch-action: pan-y;
  }

  svg:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: var(--radius);
  }

  .grid line {
    stroke: var(--border);
    stroke-width: 1;
  }

  .grid text {
    fill: var(--muted);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }

  path {
    fill: none;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .member {
    stroke: var(--muted);
    stroke-opacity: 0.35;
    stroke-width: 1;
  }

  .median {
    stroke: var(--text);
    stroke-width: 2;
  }

  .yours {
    stroke: var(--accent);
    stroke-width: 2;
  }

  .chosen {
    stroke: var(--highlight);
    stroke-width: 2;
  }

  .end {
    fill: var(--accent);
    stroke: var(--card);
    stroke-width: 2;
  }

  .end.chosen {
    fill: var(--highlight);
  }

  .crosshair {
    stroke: var(--border-strong);
    stroke-width: 1;
  }

  .tooltip {
    position: absolute;
    top: 8px;
    min-width: 150px;
    padding: 6px 10px;
    background: var(--card);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: 0 4px 12px rgb(0 0 0 / 12%);
    color: var(--text-2);
    font-size: 0.75rem;
    pointer-events: none;
    transform: translateX(12px);
  }

  .tooltip.left {
    transform: translateX(calc(-100% - 12px));
  }

  .tooltip p {
    display: flex;
    gap: 6px;
    align-items: center;
    margin: 0;
    white-space: nowrap;
  }

  .tooltip b {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }

  .tooltip .day {
    margin-bottom: 2px;
    color: var(--muted);
  }
</style>
