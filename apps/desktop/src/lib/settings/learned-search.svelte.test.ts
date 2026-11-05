import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import LearnedSearchSettings from './LearnedSearchSettings.svelte';
import { i18n } from '$lib/i18n/index.svelte';
import type { EmbeddingState, Settings, SettingsTransport } from './types';
const settings: Settings = {
	ai: {
		enabled: false,
		provider: 'Ollama',
		endpoint: 'http://127.0.0.1:11434',
		model: 'model',
	},
	tokenizer: 'Unicode61RemoveDiacritics',
	has_api_key: false,
	model_cached: false,
	experimental_semantic_search: true,
};
function fixture() {
	const transport: SettingsTransport = {
		get: vi.fn(),
		save: vi.fn(),
		tokenizer: vi.fn(),
		setExperimentalSemanticSearch: vi.fn(),
		download: vi.fn(),
		summarize: vi.fn(),
		embeddingState: vi.fn().mockResolvedValue({
			status: 'Rebuilding',
			indexed: 3,
			pending: 2,
			failed: 0,
		}),
		rebuildEmbeddings: vi.fn().mockResolvedValue(undefined),
	};
	const target = document.createElement('div');
	document.body.append(target);
	return { transport, target };
}
afterEach(() => {
	document.body.replaceChildren();
	i18n.setLocale('en');
	vi.restoreAllMocks();
});
it('shows local progress and requires explicit rebuild confirmation', async () => {
	const { target, transport } = fixture();
	const reload = vi.fn();
	const component = mount(LearnedSearchSettings, {
		target,
		props: { settings, transport, onReload: reload },
	});
	await vi.waitFor(() => expect(target.textContent).toContain('Indexed: 3'));
	expect(transport.download).not.toHaveBeenCalled();
	expect(transport.rebuildEmbeddings).not.toHaveBeenCalled();
	const button = target.querySelector('button')!;
	expect(button.disabled).toBe(true);
	const confirm = target.querySelectorAll<HTMLInputElement>(
		'input[type=checkbox]',
	)[1];
	confirm.checked = true;
	confirm.dispatchEvent(new Event('change', { bubbles: true }));
	flushSync();
	button.click();
	await vi.waitFor(() =>
		expect(transport.rebuildEmbeddings).toHaveBeenCalledExactlyOnceWith(true),
	);
	expect(reload).toHaveBeenCalledOnce();
	await unmount(component);
});
it('cleans polling and ignores an operation completing after teardown', async () => {
	const { target, transport } = fixture();
	const reload = vi.fn();
	let resolve!: () => void;
	transport.rebuildEmbeddings = vi.fn().mockReturnValue(
		new Promise<void>((done) => {
			resolve = done;
		}),
	);
	const component = mount(LearnedSearchSettings, {
		target,
		props: { settings, transport, onReload: reload },
	});
	flushSync();
	const confirm = target.querySelectorAll<HTMLInputElement>(
		'input[type=checkbox]',
	)[1];
	confirm.checked = true;
	confirm.dispatchEvent(new Event('change', { bubbles: true }));
	flushSync();
	target.querySelector('button')!.click();
	flushSync();
	expect(target.querySelector('button')!.disabled).toBe(true);
	await unmount(component);
	const calls = vi.mocked(transport.embeddingState!).mock.calls.length;
	resolve();
	await tick();
	expect(reload).not.toHaveBeenCalled();
	expect(vi.mocked(transport.embeddingState!).mock.calls.length).toBe(calls);
});
it('localizes readiness and keeps native error details private', async () => {
	i18n.setLocale('pt-BR');
	const { target, transport } = fixture();
	transport.embeddingState = vi.fn().mockResolvedValue({
		status: 'MissingModelOrRuntime',
		indexed: 0,
		pending: 2,
		failed: 0,
	} satisfies EmbeddingState);
	transport.setExperimentalSemanticSearch = vi
		.fn()
		.mockRejectedValue('private backend detail');
	const component = mount(LearnedSearchSettings, {
		target,
		props: { settings, transport },
	});
	await vi.waitFor(() =>
		expect(target.textContent).toContain('Instale o modelo verificado'),
	);
	const checkbox = target.querySelector<HTMLInputElement>('input')!;
	checkbox.checked = false;
	checkbox.dispatchEvent(new Event('change', { bubbles: true }));
	await vi.waitFor(() =>
		expect(target.querySelector('[role=alert]')?.textContent).toContain(
			'Não foi possível',
		),
	);
	expect(target.textContent).not.toContain('private backend detail');
	await unmount(component);
});
