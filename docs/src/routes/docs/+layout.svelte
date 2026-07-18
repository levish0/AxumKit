<script lang="ts">
	import { DocsLayout } from '@levish0/lily-pad';

	let { children } = $props();
</script>

<DocsLayout>
	{@render children()}
</DocsLayout>
