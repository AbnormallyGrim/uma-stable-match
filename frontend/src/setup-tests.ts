import '@testing-library/jest-dom/vitest';

// jsdom has no ResizeObserver, which `bind:clientWidth` uses. Sizes stay 0 in tests.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};

// Stubs canvas `getContext`, which some UI dependencies probe during render and jsdom lacks.
if (globalThis.HTMLCanvasElement) {
  Object.defineProperty(globalThis.HTMLCanvasElement.prototype, 'getContext', {
    value: () => null,
    configurable: true,
  });
}
