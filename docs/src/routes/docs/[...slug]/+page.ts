import { loader, localeOfUrl } from '$lib/lily-pad.js';
import type { EntryGenerator, PageLoad } from './$types.js';

export const prerender = true;

export const entries: EntryGenerator = () => loader.entries();

export const load: PageLoad = async ({ params, url }) => {
	const doc = await loader.getDoc(params.slug, localeOfUrl(url));
	return { ...doc };
};
