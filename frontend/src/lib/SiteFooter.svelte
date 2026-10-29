<!--
  @component
  The page footer: tips for switching clubs, the logo, a link to report a problem, and the
  disclaimer and attributions. Keep the wording in step with the root README's Disclaimer section.
-->

<script lang="ts">
  import carats from '../assets/carats/carats.webp';
  import BrandLogo from './BrandLogo.svelte';
  import { t } from './i18n';

  const REPO = 'https://github.com/AbnormallyGrim/uma-stable-match';
  const KOFI = 'https://ko-fi.com/L5P6299MV1';
  const FREE_CARATS = 'https://youtu.be/dQw4w9WgXcQ';

  let { today = new Date() }: { today?: Date } = $props();
  const open = (url: string) => window.open(url, '_blank', 'noopener,noreferrer');

  const aprilFools = $derived(today.getMonth() === 3 && today.getDate() === 1);
</script>

<!-- eslint-disable svelte/no-at-html-tags -- first-party text from the message catalog -->
<footer class="site-footer">
  <details class="tips">
    <summary>{t('footer.tipsSummary')}</summary>
    <ul>
      <li>{@html t('footer.tip14')}</li>
      <li>{@html t('footer.tip24')}</li>
      <li>{@html t('footer.tipLock')}</li>
      <li>{@html t('footer.tipActivity')}</li>
    </ul>
    <p>{t('footer.tipsNote')}</p>
  </details>

  <div class="meta">
    <div class="brand">
      <BrandLogo size={32} />
      <span>{t('app.name')}</span>
    </div>

    <nav aria-label={t('footer.links')}>
      <button type="button" onclick={() => open(`${REPO}/issues`)}>{t('footer.report')}</button>
      <button type="button" class="kofi" onclick={() => open(KOFI)}
        ><span aria-hidden="true">🥕</span> {t('footer.kofi')}</button
      >
      {#if aprilFools}
        <button type="button" class="carats" onclick={() => open(FREE_CARATS)}
          ><img src={carats} alt="" width="20" height="20" /> {t('footer.carats')}</button
        >
      {/if}
    </nav>
    <p class="support">{t('footer.support')}</p>

    <div class="notes">
      <p>{@html t('footer.unofficial')}</p>
      <p>{@html t('footer.data')}</p>
      <p>{@html t('footer.trademark')}</p>
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

  .site-footer :global(strong) {
    color: var(--text-2);
  }
</style>
