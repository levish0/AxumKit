<script lang="ts">
	import { DocPage } from '@levish0/lily-pad';

	let { data } = $props();
</script>

<DocPage doc={data.metadata} component={data.component} />
