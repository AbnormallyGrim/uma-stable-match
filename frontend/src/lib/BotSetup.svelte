<!--
  @component
  How a club's leader adds the bot to the club's Discord server: what taking applications does,
  then the steps, ticked as the page can tell they're done. Only the invite can be checked before
  Discord; the other steps are done once the bot is added.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { hasInvite, optInUrl } from './applications';
  import DiscordIcon from './DiscordIcon.svelte';
  import { t } from './i18n';

  interface Props {
    clubId: string;
    /** The club's name, or `null` while it's unknown. */
    clubName: string | null;
  }

  let { clubId, clubName }: Props = $props();

  /** `unknown` when the check failed: adding the bot still checks the invite. */
  type Invite = 'checking' | 'found' | 'missing' | 'unknown';
  let invite = $state<Invite>('checking');

  const flow = [
    ['leader.flow.applyTitle', 'leader.flow.applyText'],
    ['leader.flow.ticketTitle', 'leader.flow.ticketText'],
    ['leader.flow.decideTitle', 'leader.flow.decideText'],
  ] as const;

  async function check(signal?: AbortSignal) {
    invite = 'checking';
    try {
      invite = (await hasInvite(clubId, { signal })) ? 'found' : 'missing';
    } catch {
      if (!signal?.aborted) {
        invite = 'unknown';
      }
    }
  }

  onMount(() => {
    const checking = new AbortController();
    void check(checking.signal);
    return () => checking.abort();
  });
</script>

<section class="card explainer" aria-labelledby="setup-heading">
  <p class="eyebrow">{t('leader.setup.eyebrow')}</p>
  <h2 id="setup-heading">
    {t('leader.setup.heading', { name: clubName ?? t('leader.setup.yourClub') })}
  </h2>
  <p class="lede">{t('leader.setup.lede')}</p>
  <ol class="flow">
    {#each flow as [title, text], index (title)}
      <li>
        <span class="num" aria-hidden="true">{index + 1}</span>
        <strong>{t(title)}</strong>
        <span>{t(text)}</span>
      </li>
    {/each}
  </ol>
</section>

<section class="card" aria-labelledby="steps-heading">
  <h2 id="steps-heading">{t('leader.steps.heading')}</h2>
  <ol class="steps">
    <li class:done={invite === 'found'} class:current={invite !== 'found'}>
      <span
        class="box"
        role="img"
        aria-label={t(invite === 'found' ? 'leader.steps.done' : 'leader.steps.todo')}
      ></span>
      <div class="text">
        <strong>{t('leader.steps.invite')}</strong>
        {#if invite === 'found'}
          <p>{t('leader.steps.inviteFound')}</p>
        {:else}
          <p>{t('leader.steps.inviteHow')}</p>
          {#if invite === 'checking'}
            <p class="muted" role="status">{t('leader.steps.checking')}</p>
          {:else}
            {#if invite === 'missing'}
              <p class="warning" role="status">{t('leader.steps.inviteMissing')}</p>
            {/if}
            <button type="button" onclick={() => check()}>{t('leader.steps.checkAgain')}</button>
          {/if}
        {/if}
      </div>
    </li>
    <li class:current={invite === 'found'}>
      <span class="box" role="img" aria-label={t('leader.steps.todo')}></span>
      <div class="text">
        <strong>{t('leader.steps.permissions')}</strong>
        <p>{t('leader.steps.permissionsHow')}</p>
      </div>
    </li>
    <li class="add" class:current={invite === 'found'}>
      <span class="box" role="img" aria-label={t('leader.steps.todo')}></span>
      <div class="text">
        <strong>{t('leader.steps.addBot')}</strong>
        <p>{t('leader.steps.addBotHow')}</p>
      </div>
      {#if invite === 'checking' || invite === 'missing'}
        <button class="provider-btn discord" type="button" disabled>
          <span class="provider-icon"><DiscordIcon /></span>{t('leader.addBot')}
        </button>
      {:else}
        <a class="provider-btn discord" href={optInUrl(clubId)}>
          <span class="provider-icon"><DiscordIcon /></span>{t('leader.addBot')}
        </a>
      {/if}
    </li>
  </ol>
</section>

<style>
  .card {
    padding: 24px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  h2 {
    margin: 0 0 16px;
    font-size: 1.15rem;
  }

  .explainer {
    text-align: center;
  }

  .explainer h2 {
    margin: 4px 0 8px;
    font-size: 1.5rem;
  }

  .eyebrow {
    margin: 0;
    color: var(--accent);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .lede {
    max-width: 620px;
    margin: 0 auto;
    color: var(--muted);
  }

  .flow {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 16px;
    margin: 24px 0 0;
    padding: 0;
    list-style: none;
    text-align: left;
  }

  .flow li {
    display: grid;
    gap: 4px;
    padding: 16px;
    background: var(--surface);
    border-radius: var(--radius);
  }

  .flow span:last-child {
    color: var(--muted);
  }

  .num {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 800;
  }

  .steps {
    display: grid;
    gap: 20px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .steps li {
    display: grid;
    grid-template-columns: 24px 1fr;
    gap: 6px 12px;
    align-items: start;
  }

  .text p {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 0.9rem;
  }

  .text button {
    margin-top: 10px;
  }

  .text .warning {
    color: var(--approval);
    font-weight: 600;
  }

  .box {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    margin-top: 1px;
    border: 2px solid var(--control-border);
    border-radius: 50%;
  }

  .current .box {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 25%, transparent);
  }

  .done .box {
    background: var(--open);
    border-color: var(--open);
  }

  .done .box::after {
    content: '✓';
    color: #ffffff;
    font-size: 0.8rem;
    font-weight: 800;
  }

  .done strong {
    color: var(--muted);
  }

  /* The Discord button sits under step 3 on phones, and beside it with room. */
  .add .provider-btn {
    grid-column: 2;
    justify-self: start;
    margin-top: 6px;
  }

  @media (width >= 720px) {
    .add {
      grid-template-columns: 24px 1fr auto;
    }

    .add .provider-btn {
      grid-column: 3;
      align-self: center;
      margin-top: 0;
    }
  }
</style>
