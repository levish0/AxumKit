import { paraglideVitePlugin } from '@inlang/paraglide-js';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import adapter from '@sveltejs/adapter-cloudflare';
import { sveltekit } from '@sveltejs/kit/vite';
import { lilyPadPreprocess } from '@levish0/lily-pad/vite';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				// Force runes for our own code. Skip node_modules and let mdsx-generated
				// `.md` components auto-detect. Can be removed in svelte 6.
				runes: ({ filename }) => {
					if (filename.split(/[/\\]/).includes('node_modules')) return undefined;
					if (filename.endsWith('.md')) return undefined;
					return true;
				}
			},
			adapter: adapter({
				routes: {
					// The pagefind index is written into the output dir after the
					// adapter runs — serve it as static assets, not through the worker.
					exclude: ['<all>', '/pagefind/*']
				}
			}),
			preprocess: lilyPadPreprocess(),
			extensions: ['.svelte', '.md'],
			alias: {
				'$content/*': '.velite/*',
				$config: 'lily-pad.config.ts'
			}
		}),

		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide',
			strategy: ['url']
		})
	],
	server: {
		fs: {
			allow: ['.velite', 'content']
		}
	}
});
