import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

type NodeProcessLike = {
	stdout?: { isTTY?: boolean };
	env?: Record<string, string | undefined>;
};

const nodeProcess = (globalThis as { process?: NodeProcessLike }).process;
const isInteractiveTty =
	Boolean(nodeProcess?.stdout?.isTTY) &&
	!nodeProcess?.env?.CI &&
	nodeProcess?.env?.TERM !== 'dumb';

export default defineConfig(({ mode }) => ({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		port: 1420,
		strictPort: true
	},
	...(mode === 'test' ? { resolve: { conditions: ['browser'] } } : {}),
	test: {
		environment: 'jsdom',
		setupFiles: ['./src/vitest-setup.ts'],
		include: ['src/**/*.{test,spec}.{js,ts}'],
		pool: 'forks',
		maxConcurrency: 1,
		fileParallelism: false,
		reporters: isInteractiveTty ? ['default'] : [['default', { summary: false }]]
	}
}));
