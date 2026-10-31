import { mount } from 'svelte';

import './app.css';
import App from './App.svelte';
import ClubPage from './lib/ClubPage.svelte';
import { clubRoute, isVoteRoute } from './lib/route';
import VotePage from './lib/VotePage.svelte';

const target = document.getElementById('app');
if (!target) {
  throw new Error('missing #app element');
}

const club = clubRoute(window.location.pathname, window.location.search);

export default club
  ? mount(ClubPage, { target, props: { route: club } })
  : isVoteRoute(window.location.pathname)
    ? mount(VotePage, { target })
    : mount(App, { target });
