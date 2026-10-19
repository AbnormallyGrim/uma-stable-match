import { render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import App from './App.svelte';

function stubFetch(result: Promise<Response>) {
  const fetch = vi.fn(() => result);
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('App', () => {
  it('checks the API health endpoint', async () => {
    const fetch = stubFetch(Promise.resolve(new Response('{"status":"ok"}')));
    render(App);
    expect(screen.getByTestId('health')).toHaveTextContent('checking');
    expect(await screen.findByText('online')).toBeInTheDocument();
    expect(fetch).toHaveBeenCalledWith('/api/v1/health');
  });

  it('shows unreachable for an error status', async () => {
    stubFetch(Promise.resolve(new Response('', { status: 503 })));
    render(App);
    expect(await screen.findByText('unreachable')).toBeInTheDocument();
  });

  it('shows unreachable when the request fails', async () => {
    stubFetch(Promise.reject(new TypeError('network error')));
    render(App);
    expect(await screen.findByText('unreachable')).toBeInTheDocument();
  });
});
