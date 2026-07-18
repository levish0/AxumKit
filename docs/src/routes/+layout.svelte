<script lang="ts">
	import type { Pathname } from '$app/types';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { AppShell, SiteFooter } from '@levish0/lily-pad';
	import { locales, localizeHref } from '$lib/paraglide/runtime';
	import { docs, runtime, site } from '$lib/lily-pad.js';
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<AppShell {site} {runtime} {docs}>
	{@render children()}
	<!-- Renders only when `footer` is set in lily-pad.config.ts — move or replace freely. -->
	<SiteFooter />
</AppShell>

<div style="display:none">
	{#each locales as locale (locale)}
		<a href={resolve(localizeHref(page.url.pathname, { locale }) as Pathname)}>{locale}</a>
	{/each}
</div>
