import { useSyncExternalStore } from 'react';
import { zh } from './zh';

export type Locale = 'en' | 'zh';
const KEY = 'mathesis.language';
const listeners = new Set<() => void>();
function storedLocale(): Locale {
  try {
    return window.localStorage.getItem(KEY) === 'zh' ? 'zh' : 'en';
  } catch {
    return 'en';
  }
}
let locale = storedLocale();
function applyLanguage() {
  if (typeof document !== 'undefined')
    document.documentElement.lang = locale === 'zh' ? 'zh-Hans' : 'en';
}
applyLanguage();
export function getLocale(): Locale {
  return locale;
}
export function setLocale(next: Locale) {
  try {
    window.localStorage.setItem(KEY, next);
  } catch {
    /* The in-memory preference also works without storage. */
  }
  updateLocale(next);
}
function updateLocale(next: Locale) {
  locale = next;
  applyLanguage();
  listeners.forEach((listener) => listener());
}
if (typeof window !== 'undefined')
  window.addEventListener('storage', (event) => {
    if (event.key === KEY || event.key === null) updateLocale(storedLocale());
  });
function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
export function useLocale(): Locale {
  return useSyncExternalStore(subscribe, getLocale, () => 'en');
}
/** Only interface copy is passed here; manuscript text and source identities stay untouched. */
export function t(english: string, values?: Record<string, string | number>): string {
  const key = english.trim();
  const translation = locale === 'zh' && Object.hasOwn(zh, key) ? zh[key] : undefined;
  const message = translation ? english.replace(key, () => translation) : english;
  return values
    ? message.replace(/\{(\w+)\}/g, (match, key: string) => String(values[key] ?? match))
    : message;
}
