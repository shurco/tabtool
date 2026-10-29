import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: {
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter: adapter({
				pages: 'build',
				assets: 'build',
				precompress: false
			}),
			prerender: {
				entries: ['*'],
				handleHttpError: 'fail'
			},
			paths: {
				origin: 'https://tabtool.app'
			}
		})
	]
});
