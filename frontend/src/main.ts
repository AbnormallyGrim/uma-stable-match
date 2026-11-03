import * as Sentry from '@sentry/svelte';
import { mount } from 'svelte';

import './app.css';
import App from './App.svelte';
import ClubPage from './lib/ClubPage.svelte';
import { clubRoute, isWishlistRoute } from './lib/route';
import WishlistPage from './lib/WishlistPage.svelte';

// The DSN is a public key: it only lets a page send events to this project.
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

const club = clubRoute(window.location.pathname, window.location.search);

export default club
  ? mount(ClubPage, { target, props: { route: club } })
  : isWishlistRoute(window.location.pathname)
    ? mount(WishlistPage, { target })
    : mount(App, { target });
