<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import InboxView from '$lib/inbox/InboxView.svelte';
	import { InboxController } from '$lib/inbox/controller.svelte';
	import { createRows, transport, reader, pageCalls } from './performance.fixtures';
	const inbox = new InboxController(transport);
	const pause = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));
	const frame = () => new Promise<number>((resolve) => requestAnimationFrame(resolve));
	const report = (data: Record<string, unknown>) => invoke('performance_report', { report: data });
	async function scroll(label: string, frames: number) {
		const viewport = document.querySelector<HTMLElement>('[role="listbox"]');
		if (!viewport) throw new Error('Missing message viewport');
		const intervals: number[] = [];
		let maxMounted = 0;
		let previous = await frame();
		for (let i = 0; i < frames; i++) {
			viewport.scrollTop =
				(i * 40_000) % Math.max(1, viewport.scrollHeight - viewport.clientHeight);
			const now = await frame();
			intervals.push(now - previous);
			previous = now;
			maxMounted = Math.max(maxMounted, document.querySelectorAll('[role="option"]').length);
		}
		await report({
			phase: label,
			intervals_ms: intervals,
			max_mounted: maxMounted,
			loaded: inbox.rows.length,
			page_calls: pageCalls,
			scroll_height: viewport.scrollHeight,
			viewport: [innerWidth, innerHeight],
			user_agent: navigator.userAgent
		});
	}
	async function run() {
		await report({ phase: 'idle', rows: inbox.rows.length });
		await pause(5000);
		// INVARIANT: Preloaded summaries measure virtualization, not database/IPC or ingestion.
		inbox.rows = createRows();
		inbox.hasMore = false;
		inbox.syncStatus = 'Synthetic local fixture';
		await tick();
		await report({ phase: 'loaded', rows: inbox.rows.length });
		await pause(5000);
		await scroll('preloaded-scroll', 180);
		await inbox.connect();
		await tick();
		await scroll('paged-scroll', 180);
		await report({ phase: 'complete', loaded: inbox.rows.length, page_calls: pageCalls });
	}
	onMount(() => {
		void run().catch((error: unknown) => report({ phase: 'error', error: String(error) }));
		return () => inbox.dispose();
	});
</script>

<InboxView {inbox} connected={true} readerTransport={reader} />
