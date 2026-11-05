<script lang="ts">
	import { onDestroy } from 'svelte';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { SettingsTransport } from './types';
	let {
		cached,
		transport,
		changed,
	}: { cached: boolean; transport: SettingsTransport; changed: () => void } =
		$props();
	let busy = $state(false);
	let result = $state<'ok' | 'error' | ''>('');
	let disposed = false;
	onDestroy(() => {
		disposed = true;
	});
	const pt = $derived(i18n.locale === 'pt-BR');
	const text = $derived(
		pt
			? {
					title: 'Arquivos do modelo local',
					cached: 'Arquivos verificados',
					missing: 'Arquivos ausentes ou inválidos',
					info: 'MiniLM neural em CPU. Requer o runtime ONNX verificado, instalado separadamente. A busca precisa ser ativada nas configurações de busca.',
					model: 'URL HTTPS direta do modelo ONNX',
					tokenizer: 'URL HTTPS direta do tokenizer.json',
					policy:
						'Somente a revisão MiniLM fixada no guia é aceita. Limites: modelo 128 MiB, tokenizer 8 MiB. Sem redirecionamentos ou credenciais na URL. Cada arquivo tem prazo de dois minutos. Os servidores recebem seu IP, nunca suas mensagens.',
					consent: 'Autorizo baixar os dois arquivos dessas URLs.',
					download: 'Baixar arquivos do modelo',
					downloading: 'Baixando…',
					ok: 'Arquivos do modelo verificados.',
					error:
						'Falha no download ou na verificação. Confira as URLs diretas e os arquivos da revisão suportada.',
				}
			: {
					title: 'Local model files',
					cached: 'Files verified',
					missing: 'Files missing or invalid',
					info: 'Neural MiniLM on CPU. Requires the separately installed verified ONNX runtime. Enable indexing in search settings.',
					model: 'Direct ONNX model HTTPS URL',
					tokenizer: 'Direct tokenizer.json HTTPS URL',
					policy:
						'Only the pinned MiniLM revision in the guide is accepted. Limits: 128 MiB model, 8 MiB tokenizer. No redirects or URL credentials. Each file has a two-minute deadline. Hosts receive your IP, never your messages.',
					consent: 'I consent to download both files from these URLs.',
					download: 'Download model files',
					downloading: 'Downloading…',
					ok: 'Model files verified.',
					error:
						'Download or verification failed. Check direct URLs and supported revision files.',
				},
	);
	async function download(event: SubmitEvent) {
		event.preventDefault();
		if (busy || cached) return;
		const form = event.currentTarget as HTMLFormElement;
		const data = new FormData(form);
		if (data.get('consent') !== 'on') return;
		busy = true;
		result = '';
		try {
			await transport.download({
				model_url: String(data.get('modelUrl')),
				tokenizer_url: String(data.get('tokenizerUrl')),
				consent: true,
			});
			if (!disposed) {
				result = 'ok';
				form.reset();
				changed();
			}
		} catch {
			if (!disposed) result = 'error';
		} finally {
			if (!disposed) busy = false;
		}
	}
</script>

<section class="space-y-4" aria-label={text.title}>
	<h2 class="text-lg font-semibold">{text.title}</h2>
	<p class="text-sm text-[var(--secondary)]">
		all-MiniLM-L6-v2 · {cached ? text.cached : text.missing}.
		$XDG_DATA_HOME/triage-mail/models
	</p>
	<p class="text-sm text-[var(--secondary)]">{text.info}</p>
	{#if !cached}
		<form onsubmit={download} class="space-y-4">
			<label class="block"
				>{text.model}<input
					name="modelUrl"
					type="url"
					required
					disabled={busy}
					class="mt-2 block w-full rounded border p-2"
				/></label
			>
			<label class="block"
				>{text.tokenizer}<input
					name="tokenizerUrl"
					type="url"
					required
					disabled={busy}
					class="mt-2 block w-full rounded border p-2"
				/></label
			>
			<p class="text-xs text-[var(--secondary)]">{text.policy}</p>
			<label class="flex items-start gap-2"
				><input
					name="consent"
					type="checkbox"
					required
					disabled={busy}
				/>{text.consent}</label
			>
			<button
				type="submit"
				disabled={busy}
				class="rounded-lg bg-[var(--selection-soft)] px-4 py-2 text-[var(--accent)]"
				>{busy ? text.downloading : text.download}</button
			>
		</form>
	{/if}
	{#if result}<p role="status" class="text-sm">{text[result]}</p>{/if}
</section>
