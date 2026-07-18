import { createDocLoader, type DocEntry, type LocaleRuntime } from '@levish0/lily-pad';
import { docs as veliteDocs } from '$content/index.js';
import {
	baseLocale,
	deLocalizeHref,
	extractLocaleFromUrl,
	locales,
	localizeHref
} from '$lib/paraglide/runtime';
import { site } from '$config';

export { site };

export const docs = veliteDocs as DocEntry[];

/** paraglide's runtime, narrowed to what the lily-pad theme needs. */
export const runtime: LocaleRuntime = {
	locales,
	baseLocale,
	localizeHref,
	deLocalizeHref,
	extractLocaleFromUrl
};

export const loader = createDocLoader(import.meta.glob('/content/**/*.md'), docs, baseLocale);

/** Locale for a URL, safe on both server and client. */
export function localeOfUrl(url: URL | string) {
	return extractLocaleFromUrl(url) ?? baseLocale;
}
