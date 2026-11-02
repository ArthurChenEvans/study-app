import "@testing-library/jest-dom/vitest";

// jsdom has no layout engine: give components that measure themselves a no-op observer.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
} as unknown as typeof ResizeObserver;
