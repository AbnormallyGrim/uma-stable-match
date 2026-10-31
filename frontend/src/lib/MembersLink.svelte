<!--
  @component
  A button-styled link to a club's page: "See how you fit" after a trainer search, which places the
  trainer among the members, or "See members" after a pace search. It notes the search's address
  first, so the club page's "Back to search" returns to these results.
-->

<script lang="ts">
  import { t } from './i18n';
  import { clubPath, rememberSearch } from './route';

  interface Props {
    circleId: number;
    name: string;
    /** The trainer searched for, or `null` for a pace search. */
    trainerId: string | null;
    /** A smaller button, for table rows. */
    compact?: boolean;
  }

  let { circleId, name, trainerId, compact = false }: Props = $props();

  const storage = typeof sessionStorage === 'undefined' ? undefined : sessionStorage;
  const label = $derived(trainerId === null ? t('club.seeMembers') : t('club.seeFit'));
  const spoken = $derived(
    trainerId === null ? t('club.seeMembersOf', { name }) : t('club.seeFitIn', { name }),
  );
</script>

<a
  class="members"
  class:compact
  href={clubPath(circleId, trainerId)}
  aria-label={spoken}
  onclick={() => rememberSearch(storage, window.location.href)}
  >{label} <span aria-hidden="true">→</span></a
>

<style>
  .members {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    justify-content: center;
    padding: 6px 12px;
    background: color-mix(in srgb, var(--accent) 10%, var(--card));
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    color: var(--accent);
    font-size: 0.8125rem;
    font-weight: 600;
    text-decoration: none;
    white-space: nowrap;
  }

  .members:hover {
    background: color-mix(in srgb, var(--accent) 15%, var(--card));
  }

  .members:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .compact {
    padding: 2px 8px;
    font-size: 0.75rem;
  }
</style>
