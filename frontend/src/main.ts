import { mount } from 'svelte';

import './app.css';
import App from './App.svelte';
import ClubPage from './lib/ClubPage.svelte';
import { clubRoute, isWishlistRoute } from './lib/route';
import WishlistPage from './lib/WishlistPage.svelte';

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
