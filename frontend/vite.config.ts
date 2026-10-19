import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vitest/config';

const apiTarget = process.env.API_TARGET ?? 'http://127.0.0.1:8080';

export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  server: {
    proxy: {
      '/api': apiTarget,
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
    setupFiles: ['./src/setup-tests.ts'],
  },
});
