<script lang="ts">
	import { onDestroy } from 'svelte';
	import { i18n } from '$lib/i18n/index.svelte';
	import type { EmbeddingState, SettingsModuleProps } from './types';
	let {
		settings,
		transport,
		busy = false,
		onReload,
	}: SettingsModuleProps = $props();
	let progress = $state<EmbeddingState | null>(null);
	let working = $state(false);
	let error = $state(false);
	let disposed = false;
	onDestroy(() => {
		disposed = true;
	});
	const pt = $derived(i18n.locale === 'pt-BR');
	const labels = $derived(
		pt
			? {
					title: 'Busca semântica local',
					info: 'Embeddings neurais MiniLM no seu computador. Ativar inicia a indexação do texto em cache; nenhuma mensagem é enviada e nenhum arquivo é baixado automaticamente.',
					enable: 'Ativar busca semântica',
					rebuild: 'Reconstruir índice local',
					confirm:
						'Confirmo a reconstrução. A busca semântica ficará indisponível até a conclusão.',
					error: 'Não foi possível atualizar o índice local.',
					indexed: 'Indexadas',
					pending: 'Pendentes',
					failed: 'Falhas',
					states: {
						Disabled: 'Desativada',
						MissingModelOrRuntime:
							'Instale o modelo verificado e o runtime CPU. Consulte o guia de configuração.',
						Rebuilding: 'Indexando texto em cache…',
						Failed: 'Falha na indexação. Reconstrua para tentar novamente.',
						Ready: 'Pronta',
						Unavailable: 'Estado indisponível',
					},
				}
			: {
					title: 'Local semantic search',
					info: 'Neural MiniLM embeddings run on your computer. Enabling starts indexing cached text; no messages are sent and no files are downloaded automatically.',
					enable: 'Enable semantic search',
					rebuild: 'Rebuild local index',
					confirm:
						'I confirm rebuilding. Semantic search will be unavailable until it finishes.',
					error: 'The local index could not be updated.',
					indexed: 'Indexed',
					pending: 'Pending',
					failed: 'Failed',
					states: {
						Disabled: 'Disabled',
						MissingModelOrRuntime:
							'Install the verified model and CPU runtime. See the setup guide.',
						Rebuilding: 'Indexing cached text…',
						Failed: 'Indexing failed. Rebuild to retry.',
						Ready: 'Ready',
						Unavailable: 'Status unavailable',
					},
				},
	);
	let confirm = $state(false);
	$effect(() => {
		settings.experimental_semantic_search;
		let active = true;
		let loading = false;
		progress = null;
		async function refresh() {
			if (loading || !transport.embeddingState) return;
			loading = true;
			try {
				const next = await transport.embeddingState();
				if (active) progress = next;
			} catch {
				if (active) progress = null;
			} finally {
				loading = false;
			}
		}
		void refresh();
		const timer = setInterval(() => void refresh(), 2000);
		return () => {
			active = false;
			clearInterval(timer);
		};
	});
	async function change(enabled: boolean, rebuild = false) {
		if (busy || working || (rebuild && !confirm)) return;
		working = true;
		error = false;
		try {
			if (rebuild) {
				if (!transport.rebuildEmbeddings) throw new Error('unavailable');
				await transport.rebuildEmbeddings(true);
			} else await transport.setExperimentalSemanticSearch(enabled);
			if (!disposed) {
				confirm = false;
				await onReload?.();
			}
		} catch {
			if (!disposed) error = true;
		} finally {
			if (!disposed) working = false;
		}
	}
</script>

<section class="space-y-4" aria-label={labels.title}>
	<h3 class="text-base font-medium text-[var(--text)]">{labels.title}</h3>
	<p class="text-sm text-[var(--secondary)]">{labels.info}</p>
	<label class="flex items-center gap-3 text-sm">
		<input
			type="checkbox"
			checked={settings.experimental_semantic_search}
			disabled={busy || working}
			onchange={(e) => void change(e.currentTarget.checked)}
		/>{labels.enable}
	</label>
	<p role="status" class="text-sm">
		{labels.states[progress?.status ?? 'Unavailable']}
	</p>
	{#if progress}
		<p class="text-sm tabular-nums">
			{labels.indexed}: {progress.indexed} · {labels.pending}: {progress.pending}
			· {labels.failed}: {progress.failed}
		</p>
	{/if}
	{#if settings.experimental_semantic_search}
		<label class="flex items-start gap-2 text-sm"
			><input
				type="checkbox"
				bind:checked={confirm}
				disabled={busy || working}
			/>{labels.confirm}</label
		>
		<button
			type="button"
			disabled={!confirm || busy || working || !transport.rebuildEmbeddings}
			onclick={() => void change(true, true)}
			class="rounded-lg bg-[var(--selection-soft)] px-4 py-2 text-[var(--accent)] disabled:opacity-50"
			>{labels.rebuild}</button
		>
	{/if}
	{#if error}<p role="alert">{labels.error}</p>{/if}
</section>
