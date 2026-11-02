<script lang="ts">
	import { tick } from 'svelte';
	import { i18n } from '$lib/i18n/index.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import InboxView from '$lib/inbox/InboxView.svelte';
	import { InboxController } from '$lib/inbox/controller.svelte';
	import {
		createRows,
		transport,
		reader,
		metrics,
		accounts,
		avatars
	} from './performance.fixtures';
	const inbox = new InboxController(transport);
	const pause = (ms: number) =>
		new Promise<void>((resolve) => setTimeout(resolve, ms));
	const frame = () =>
		new Promise<number>((resolve) => requestAnimationFrame(resolve));
	const report = (data: Record<string, unknown>) =>
		'__TAURI_INTERNALS__' in window
			? invoke('performance_report', { report: data })
			: Promise.resolve();
	async function scroll(label: string, frames: number) {
		const viewport = document.querySelector<HTMLElement>('[role="listbox"]');
		if (!viewport) throw new Error('Missing message viewport');
		const intervals: number[] = [];
		let maxMounted = 0;
		let previous = await frame();
		for (let i = 0; i < frames; i++) {
			viewport.scrollTop =
				(i * 40_000) %
				Math.max(1, viewport.scrollHeight - viewport.clientHeight);
			const now = await frame();
			intervals.push(now - previous);
			previous = now;
			maxMounted = Math.max(
				maxMounted,
				document.querySelectorAll('[role="option"]').length
			);
		}
		await report({
			phase: label,
			intervals_ms: intervals,
			max_mounted: maxMounted,
			loaded: inbox.rows.length,
			page_calls: metrics.pageCalls,
			scroll_height: viewport.scrollHeight,
			viewport: [innerWidth, innerHeight],
			user_agent: navigator.userAgent
		});
	}
	async function run() {
		inbox.experimentalSemanticSearch = true;
		await inbox.connect();
		await report({ phase: 'idle', rows: inbox.rows.length });
		// INVARIANT: Timer starts at controller submission, excludes toolbar debounce and startup.
		for (const view of [
			'Inbox',
			'Starred',
			'Snoozed',
			{ Folder: 'Archive' }
		] as const) {
			for (const mode of ['Lexical', 'Semantic', 'Hybrid'] as const) {
				const submission: number[] = [];
				const response: number[] = [];
				inbox.scope = { AccountView: { account_id: 'account-2', view } };
				for (let sample = 0; sample < 35; sample++) {
					const start = performance.now();
					await inbox.search(mode === 'Lexical' ? '' : 'newsletter', mode);
					const ipc = metrics.ipc.at(-1) ?? 0;
					await tick();
					await frame();
					await frame();
					if (inbox.error) throw new Error(inbox.error);
					if (
						inbox.rows.length !== (view === 'Starred' ? 31 : 200) ||
						inbox.rows.some((row) => row.account_id !== 'account-2')
					)
						throw new Error('Incomplete or cross-account native fixture page');
					if (sample >= 5) {
						submission.push(performance.now() - start);
						response.push(ipc);
					}
				}
				await report({
					phase: 'first-page',
					view,
					mode,
					submission_to_frame_ms: submission,
					ipc_response_ms: response,
					rows: inbox.rows.length,
					mounted: document.querySelectorAll('[role="option"]').length
				});
			}
		}
		inbox.scope = undefined;
		await inbox.search('', 'Lexical');
		await pause(5000);
		// INVARIANT: Preloaded summaries measure virtualization, not database/IPC or ingestion.
		inbox.rows = createRows();
		inbox.hasMore = false;
		inbox.syncStatus = 'Synthetic local fixture';
		await tick();
		await report({ phase: 'loaded', rows: inbox.rows.length });
		await pause(5000);
		await scroll('preloaded-scroll', 180);
		await inbox.search('', 'Lexical');
		await tick();
		await scroll('paged-scroll', 180);
		await report({
			phase: 'complete',
			loaded: inbox.rows.length,
			page_calls: metrics.pageCalls
		});
	}
	$effect(() => {
		i18n.setLocale(
			new URLSearchParams(location.search).get('locale') === 'pt-BR'
				? 'pt-BR'
				: 'en'
		);
		if ('__TAURI_INTERNALS__' in window)
			void run().catch((error: unknown) =>
				report({ phase: 'error', error: String(error) })
			);
		else void inbox.connect();
		return () => inbox.dispose();
	});
</script>

<InboxView
	{inbox}
	{accounts}
	avatarTransport={avatars}
	connected={true}
	readerTransport={reader}
/>
