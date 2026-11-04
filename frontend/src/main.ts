import * as Sentry from '@sentry/svelte';
import { mount } from 'svelte';

import './app.css';
import App from './App.svelte';
import ClubPage from './lib/ClubPage.svelte';
import ApplicationsPage from './lib/ApplicationsPage.svelte';
import { APPLICATIONS_ENABLED } from './lib/flags';
import LoginPage from './lib/LoginPage.svelte';
import { applicationsRoute, clubRoute, isLoginRoute, isWishlistRoute } from './lib/route';
import SettingsPage from './lib/SettingsPage.svelte';
import WishlistPage from './lib/WishlistPage.svelte';
import { signedInUser } from './lib/applications';

Sentry.init({
  dsn: 'https://9e1efba7aae097a3e0c5df71f48e4aeb@o4512370149097472.ingest.us.sentry.io/4512370231279616',
  environment: import.meta.env.MODE,
  enabled: import.meta.env.PROD,
  tracesSampleRate: 0.1,
});

const target = document.getElementById('app');
if (!target) {
  throw new Error('missing #app element');
}

const isProtectedApplications =
  APPLICATIONS_ENABLED &&
  (window.location.pathname === '/applications' ||
    window.location.pathname === '/applications/' ||
    /^\/applications(?:\/|$)/.test(window.location.pathname));

let shouldRedirectToLogin = false;
if (isProtectedApplications && !isLoginRoute(window.location.pathname)) {
  const discordId = await signedInUser().catch(() => null);
  if (discordId === null) {
    const next = `${window.location.pathname}${window.location.search}`;
    window.location.replace(`/login?next=${encodeURIComponent(next)}`);
    shouldRedirectToLogin = true;
  }
}

if (!shouldRedirectToLogin) {
  const club = clubRoute(window.location.pathname, window.location.search);
  const applications = APPLICATIONS_ENABLED
    ? applicationsRoute(window.location.pathname, window.location.search)
    : null;

  const isApplicationHome = applications?.page === 'home';

  if (club) {
    mount(ClubPage, { target, props: { route: club } });
  } else if (applications?.page === 'settings') {
    mount(SettingsPage, {
      target,
      props: { clubId: applications.clubId, linked: applications.linked },
    });
  } else if (applications && isApplicationHome) {
    mount(ApplicationsPage, { target, props: { error: applications.error } });
  } else if (isWishlistRoute(window.location.pathname)) {
    mount(WishlistPage, { target });
  } else if (isLoginRoute(window.location.pathname)) {
    mount(LoginPage, { target });
  } else {
    mount(App, { target });
  }
}