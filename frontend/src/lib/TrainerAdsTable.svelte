<!--
  @component
  The ad a trainer posted: what they're looking for, when it was posted, whether it's on the board
  and how many invitations it has, with a link to manage it. On narrow screens the row shows as a
  card.
-->

<script lang="ts">
  import { adDay, rankRange, type MyAd } from './ads';
  import { t } from './i18n';
  import { adsPath } from './route';

  interface Props {
    /** The trainer's ad, or `null` without one. */
    ad: MyAd | null;
  }

  let { ad }: Props = $props();
</script>

<section class="card" aria-labelledby="trainer-ads-heading">
  <h2 id="trainer-ads-heading">{t('trainer.ads.heading')}</h2>
  {#if ad === null}
    <div class="empty">
      <strong>{t('trainer.ads.empty')}</strong>
      <span>{t('trainer.ads.emptyHint')}</span>
      <a href={adsPath(true)}>{t('ads.post')}</a>
    </div>
  {:else}
    <table>
      <thead>
        <tr>
          <th scope="col">{t('ads.table.wants')}</th>
          <th scope="col">{t('ads.table.posted')}</th>
          <th scope="col">{t('leader.table.status')}</th>
          <th scope="col">{t('trainer.ads.invites')}</th>
          <th scope="col"><span class="visually-hidden">{t('leader.table.actions')}</span></th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td class="wants" data-label={t('ads.table.wants')}>
            <div class="chips">
              <span>{rankRange(ad.rankMin, ad.rankMax)}</span>
              {#if ad.discord !== 'any'}
                <span class="tag">{t(`ads.table.discord.${ad.discord}`)}</span>
              {/if}
              {#if ad.language !== 'any'}
                <span class="tag">{t(`ad.language.${ad.language}`)}</span>
              {/if}
            </div>
          </td>
          <td data-label={t('ads.table.posted')}>{adDay(ad.postedAt)}</td>
          <td data-label={t('leader.table.status')}>
            <span class="tag" class:on={ad.onBoard}>
              {ad.onBoard
                ? t('trainer.ads.onBoard', { date: adDay(ad.visibleUntil) })
                : t('trainer.ads.off')}
            </span>
          </td>
          <td class="num" data-label={t('trainer.ads.invites')}>{ad.invites.length}</td>
          <td class="act">
            <a class="button-link" href={adsPath(true)}>{t('trainer.ads.manage')}</a>
          </td>
        </tr>
      </tbody>
    </table>
  {/if}
</section>

<style>
  .card {
    padding: 20px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  h2 {
    margin: 0 0 12px;
    font-size: 1.05rem;
  }

  .empty {
    display: grid;
    gap: 4px;
    padding: 24px 16px;
    color: var(--muted);
    text-align: center;
  }

  .empty strong {
    color: var(--text);
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.75rem;
    letter-spacing: 0.05em;
    text-align: left;
    text-transform: uppercase;
  }

  td {
    padding: 10px;
    vertical-align: middle;
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 6px;
    align-items: center;
  }

  .tag {
    display: inline-block;
    padding: 2px 8px;
    background: var(--surface);
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .tag.on {
    color: var(--open);
  }

  .act {
    text-align: right;
  }

  /* A link that looks like a primary button. */
  .button-link {
    display: inline-flex;
    align-items: center;
    padding: 6px 14px;
    background: var(--accent);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    color: var(--on-accent);
    font-size: 0.875rem;
    font-weight: 600;
    text-decoration: none;
    white-space: nowrap;
  }

  /* The row becomes a card, with the column names beside the values. */
  @media (width < 720px) {
    table,
    tbody,
    tr,
    td {
      display: block;
    }

    thead {
      display: none;
    }

    td {
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      gap: 12px;
      padding: 3px 0;
      text-align: right;
    }

    .chips {
      justify-content: flex-end;
    }

    td::before {
      flex: none;
      content: attr(data-label);
      color: var(--muted);
      font-size: 0.8rem;
    }

    .act {
      justify-content: flex-end;
      padding-top: 8px;
    }

    .act::before {
      display: none;
    }
  }
</style>
