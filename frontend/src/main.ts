import * as Sentry from '@sentry/svelte';
import { mount } from 'svelte';

import './app.css';
import App from './App.svelte';
import ClubPage from './lib/ClubPage.svelte';
import ApplicationsPage from './lib/ApplicationsPage.svelte';
import ClubApplicationsPage from './lib/ClubApplicationsPage.svelte';
import ClubComparePage from './lib/ClubComparePage.svelte';
import { fetchFeatures } from './lib/api';
import { flags } from './lib/flags.svelte';
import LoginPage from './lib/LoginPage.svelte';
import {
  applicationsRoute,
  clubRoute,
  isLoginRoute,
  isSettingsRoute,
  isWishlistRoute,
} from './lib/route';
import SettingsPage from './lib/SettingsPage.svelte';
import TrainerApplicationsPage from './lib/TrainerApplicationsPage.svelte';
import WishlistPage from './lib/WishlistPage.svelte';
import { loadAccount } from './lib/account.svelte';

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

// Only the applications pages wait for the flag; elsewhere it just adds a navbar link later.
const isApplicationsPath = /^\/applications(?:\/|$)/.test(window.location.pathname);
if (isApplicationsPath) {
  flags.applications = await fetchFeatures().then(
    (found) => found.applications,
    () => false,
  );
}
const isSettingsPath = isSettingsRoute(window.location.pathname);
const isProtected = (isApplicationsPath && flags.applications) || isSettingsPath;

let shouldRedirectToLogin = false;
if (isProtected) {
  if ((await loadAccount()) === null) {
    const next = `${window.location.pathname}${window.location.search}`;
    window.location.replace(`/login?next=${encodeURIComponent(next)}`);
    shouldRedirectToLogin = true;
  }
} else {
  // Elsewhere the account menu shows once the server answers.
  void loadAccount();
}

if (!shouldRedirectToLogin) {
  const club = clubRoute(window.location.pathname, window.location.search);
  const applications = flags.applications
    ? applicationsRoute(window.location.pathname, window.location.search)
    : null;

  const isApplicationHome = applications?.page === 'home';

  if (club) {
    mount(ClubPage, { target, props: { route: club } });
  } else if (applications?.page === 'club') {
    mount(ClubApplicationsPage, {
      target,
      props: { clubId: applications.clubId, linked: applications.linked },
    });
  } else if (applications?.page === 'compare') {
    mount(ClubComparePage, {
      target,
      props: { clubId: applications.clubId, trainerIds: applications.trainerIds },
    });
  } else if (applications?.page === 'trainer') {
    mount(TrainerApplicationsPage, { target });
  } else if (applications && isApplicationHome) {
    mount(ApplicationsPage, { target, props: { error: applications.error } });
  } else if (isWishlistRoute(window.location.pathname)) {
    mount(WishlistPage, { target });
  } else if (isSettingsPath) {
    mount(SettingsPage, { target });
  } else if (isLoginRoute(window.location.pathname)) {
    mount(LoginPage, { target });
  } else {
    mount(App, { target });
  }
}
