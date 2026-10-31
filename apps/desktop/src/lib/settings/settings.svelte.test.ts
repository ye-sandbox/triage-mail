import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import SettingsView from './SettingsView.svelte';
import AiSettingsForm from './AiSettingsForm.svelte';
import ModelSettings from './ModelSettings.svelte';
import MessageSummary from './MessageSummary.svelte';
import type { Settings, SettingsTransport } from './types';

const defaults = (): Settings => ({
	ai: {
		enabled: false,
		provider: 'Ollama',
		endpoint: 'http://127.0.0.1:11434',
		model: 'llama3.2'
	},
	tokenizer: 'Unicode61RemoveDiacritics',
	has_api_key: false,
	model_cached: false
});
function fixture() {
	const transport = {
		get: vi.fn<SettingsTransport['get']>().mockResolvedValue(defaults()),
		save: vi.fn<SettingsTransport['save']>().mockResolvedValue(),
		tokenizer: vi.fn<SettingsTransport['tokenizer']>().mockResolvedValue(),
		download: vi.fn<SettingsTransport['download']>().mockResolvedValue(),
		summarize: vi
			.fn<SettingsTransport['summarize']>()
			.mockResolvedValue('<script>untrusted</script>')
	};
	const target = document.createElement('div');
	document.body.append(target);
	return { target, transport };
}
function submit(form: HTMLFormElement) {
	form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
	flushSync();
}
afterEach(() => {
	document.body.replaceChildren();
	vi.restoreAllMocks();
});

it('loads settings without downloads, probes or summaries and explicitly applies the tokenizer', async () => {
	const { target, transport } = fixture();
	const component = mount(SettingsView, {
		target,
		props: { close: vi.fn(), transport }
	});
	await vi.waitFor(() => expect(target.querySelector('[name=tokenizer]')).not.toBeNull());
	expect(transport.download).not.toHaveBeenCalled();
	expect(transport.summarize).not.toHaveBeenCalled();
	expect(transport.save).not.toHaveBeenCalled();
	const select = target.querySelector<HTMLSelectElement>('[name=tokenizer]')!;
	select.value = 'Trigram';
	submit(select.closest('form')!);
	await vi.waitFor(() => expect(transport.tokenizer).toHaveBeenCalledExactlyOnceWith('Trigram'));
	await unmount(component);
});

it('clears unbound API keys before dispatch, rejects duplicate saves and clears on teardown', async () => {
	const { target, transport } = fixture();
	let resolve!: () => void;
	transport.save.mockReturnValue(new Promise<void>((done) => (resolve = done)));
	const changed = vi.fn();
	const component = mount(AiSettingsForm, {
		target,
		props: { initial: defaults().ai, hasKey: false, transport, changed }
	});
	flushSync();
	const key = target.querySelector<HTMLInputElement>('[name=apiKey]')!;
	key.value = 'synthetic-private-value';
	submit(target.querySelector('form')!);
	submit(target.querySelector('form')!);
	expect(key.value).toBe('');
	expect(transport.save).toHaveBeenCalledExactlyOnceWith(defaults().ai, 'synthetic-private-value');
	resolve();
	await tick();
	flushSync();
	key.value = 'transient-value';
	await unmount(component);
	expect(key.value).toBe('');
});

it('requires download consent and redacts errors', async () => {
	const { target, transport } = fixture();
	transport.download.mockRejectedValue('private URL data');
	const component = mount(ModelSettings, {
		target,
		props: { cached: false, transport, changed: vi.fn() }
	});
	flushSync();
	const form = target.querySelector('form')!;
	(target.querySelector('[name=modelUrl]') as HTMLInputElement).value =
		'https://models.example.com/model.onnx';
	(target.querySelector('[name=tokenizerUrl]') as HTMLInputElement).value =
		'https://models.example.com/tokenizer.json';
	submit(form);
	expect(transport.download).not.toHaveBeenCalled();
	(target.querySelector('[name=consent]') as HTMLInputElement).checked = true;
	submit(form);
	await vi.waitFor(() => expect(target.textContent).toContain('Download failed'));
	expect(transport.download).toHaveBeenCalledExactlyOnceWith({
		model_url: 'https://models.example.com/model.onnx',
		tokenizer_url: 'https://models.example.com/tokenizer.json',
		consent: true
	});
	expect(target.textContent).not.toContain('private URL data');
	await unmount(component);
});

it('requires per-message confirmation bound to displayed AI settings and renders summaries as text', async () => {
	const { target, transport } = fixture();
	const settings = defaults();
	settings.ai.enabled = true;
	transport.get.mockResolvedValue(settings);
	const component = mount(MessageSummary, {
		target,
		props: { messageId: 'message-one', transport }
	});
	flushSync();
	expect(transport.get).not.toHaveBeenCalled();
	target.querySelector<HTMLButtonElement>('button')!.click();
	await vi.waitFor(() => expect(target.textContent).toContain('Send and summarize'));
	expect(transport.summarize).not.toHaveBeenCalled();
	const confirm = Array.from(target.querySelectorAll('button')).find(
		(button) => button.textContent === 'Send and summarize'
	)!;
	confirm.click();
	confirm.click();
	await vi.waitFor(() => expect(target.textContent).toContain('<script>untrusted</script>'));
	expect(transport.summarize).toHaveBeenCalledExactlyOnceWith('message-one', true, settings.ai);
	expect(target.querySelector('script')).toBeNull();
	await unmount(component);
});

it('keeps disabled AI offline and ignores late preparation after teardown', async () => {
	const { target, transport } = fixture();
	const component = mount(MessageSummary, {
		target,
		props: { messageId: 'message-one', transport }
	});
	flushSync();
	target.querySelector<HTMLButtonElement>('button')!.click();
	await vi.waitFor(() => expect(target.textContent).toContain('Enable summaries in Settings'));
	expect(transport.summarize).not.toHaveBeenCalled();
	let resolve!: (settings: Settings) => void;
	transport.get.mockReturnValue(new Promise<Settings>((done) => (resolve = done)));
	target.querySelector<HTMLButtonElement>('button')!.click();
	await unmount(component);
	resolve({ ...defaults(), ai: { ...defaults().ai, enabled: true } });
	await tick();
	expect(target.querySelector('button')).toBeNull();
	expect(transport.summarize).not.toHaveBeenCalled();
});
