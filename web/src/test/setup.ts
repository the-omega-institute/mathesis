import { setLocale } from '../i18n/locale';
import { afterEach, beforeEach, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';

if (typeof window !== 'undefined' && !window.matchMedia) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query: string): MediaQueryList => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }),
  });
}

// jsdom does not implement scrolling; the router scrolls to the top on navigation.
if (typeof window !== 'undefined') {
  Object.defineProperty(window, 'scrollTo', { writable: true, value: vi.fn() });
}

beforeEach(() => {
  setLocale('en');
});

afterEach(() => {
  cleanup();
});
