<script lang="ts">
	import ky from 'ky';

	let result = '';
	let error = '';

	async function callApi() {
		result = '';
		error = '';

		try {
			const response = await ky.get('http://localhost:8000/v0/user/1');
			console.log('Response:', response);
			const data = await response.json();
			result = JSON.stringify(data, null, 2);
		} catch (err) {
			if (err instanceof Error) {
				error = err.message;
			} else if (err instanceof Object && 'response' in err) {
				const text = await (err as any).response.text();
				const response = err.response as Response;
				error = `HTTP ${response.status}: ${text}`;
			} else {
				error = 'Unknown error occurred';
			}
		}
	}

	async function callApiWithDifferentHeader() {
		result = '';
		error = '';

		try {
			const response = await ky.get('http://localhost:8000/v0/user/1', {
				headers: {
					'Content-Type': 'application/xml'
				}
			});
			console.log('Response:', response);
			const data = await response.json();
			result = JSON.stringify(data, null, 2);
		} catch (err) {
			if (err instanceof Error) {
				error = err.message;
			} else if (err instanceof Object && 'response' in err) {
				const text = await (err as any).response.text();
				const response = err.response as Response;
				error = `HTTP ${response.status}: ${text}`;
			} else {
				error = 'Unknown error occurred';
			}
		}
	}

	async function callApiWithCustomHeader() {
		result = '';
		error = '';

		try {
			const response = await ky.get('http://localhost:8000/v0/user/1', {
				headers: {
					'X-Custom-Header': 'CustomValue'
				}
			});
			console.log('Response:', response);
			const data = await response.json();
			result = JSON.stringify(data, null, 2);
		} catch (err) {
			if (err instanceof Error) {
				error = err.message;
			} else if (err instanceof Object && 'response' in err) {
				const text = await (err as any).response.text();
				const response = err.response as Response;
				error = `HTTP ${response.status}: ${text}`;
			} else {
				error = 'Unknown error occurred';
			}
		}
	}
</script>

<div class="flex min-h-screen items-center justify-center bg-gray-100 p-6">
	<div class="w-full max-w-4xl space-y-6 rounded-2xl bg-white p-6 shadow-xl">
		<h1 class="text-2xl font-bold text-gray-800">API Test (with ky)</h1>

		<div class="flex space-x-4">
			<button
				class="w-full rounded-lg bg-blue-600 px-4 py-2 text-white transition hover:bg-blue-700"
				on:click={callApi}
			>
				Call API (Default)
			</button>

			<button
				class="w-full rounded-lg bg-green-600 px-4 py-2 text-white transition hover:bg-green-700"
				on:click={callApiWithDifferentHeader}
			>
				Call API with XML Content-Type
			</button>

			<button
				class="w-full rounded-lg bg-yellow-600 px-4 py-2 text-white transition hover:bg-yellow-700"
				on:click={callApiWithCustomHeader}
			>
				Call API with Custom Header
			</button>
		</div>

		{#if result}
			<div class="max-h-64 overflow-x-auto rounded-md bg-gray-900 p-4 text-sm text-green-300">
				<pre>{result}</pre>
			</div>
		{/if}

		{#if error}
			<div class="rounded-md border border-red-400 bg-red-100 p-4 text-sm text-red-800">
				<strong>Error:</strong>
				{error}
			</div>
		{/if}
	</div>
</div>
