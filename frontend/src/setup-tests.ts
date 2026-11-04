import '@testing-library/jest-dom/vitest';

// jsdom has no ResizeObserver, which `bind:clientWidth` uses. Sizes stay 0 in tests.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};

// jsdom does not implement canvas APIs. Some UI dependencies probe for them during render,
// so stub the method instead of failing with noisy warnings in the test output.
if (globalThis.HTMLCanvasElement) {
  Object.defineProperty(globalThis.HTMLCanvasElement.prototype, 'getContext', {
    value: () => null,
    configurable: true,
  });
}