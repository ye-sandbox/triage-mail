import { toast } from 'svelte-sonner';
import type { TransferTransport, Definitions, Mapping } from '$lib/backup/transport';
import { appStore } from '$lib/stores/app.svelte';
import { buildImportPreviewFromJson, triggerBlobDownload } from './settings.helpers';
import type { PortableImportPreview } from './settings.fixtures';

export function exportDemoPortableDefinitions(doneMsg: string) {
	const payload = {
		schema_version: 'triage-portable-v1',
		exported_at_iso: new Date().toISOString(),
		rules: appStore.rules,
		saved_views: appStore.savedViews,
		categories: ['Security', 'Financial', 'Shopping', 'Newsletter', 'Notifications', 'Personal']
	};
	triggerBlobDownload(
		'triage-definitions.json',
		JSON.stringify(payload, null, 2),
		'application/json'
	);
	toast.success(doneMsg);
}

export async function parseDemoPortableFile(
	event: Event,
	errorMsg: string
): Promise<PortableImportPreview | null> {
	const file =
		event.currentTarget instanceof HTMLInputElement ? event.currentTarget.files?.[0] : null;
	if (!file) return null;
	try {
		return buildImportPreviewFromJson(file.name, await file.text());
	} catch {
		toast.error(errorMsg);
		return null;
	}
}

export async function runPortableAction(
	action: 'export' | 'preview' | 'import',
	transport: TransferTransport,
	state: {
		nativePreview: Definitions | null;
		nativeMapping: Mapping;
		nativePath: string;
		nativeConsent: boolean;
		nativeComplete: boolean;
	}
): Promise<
	| { kind: 'imported' }
	| { kind: 'exported' }
	| { kind: 'preview'; value: Definitions; targets: { id: string; name: string }[]; path: string }
	| { kind: 'noop' }
> {
	if (action === 'import') {
		if (!state.nativePreview || !state.nativeConsent || !state.nativeComplete) return { kind: 'noop' };
		await transport.import(state.nativePath, state.nativeMapping, state.nativePreview, true);
		if (typeof window !== 'undefined') window.dispatchEvent(new Event('triage-definitions-imported'));
		return { kind: 'imported' };
	}
	const selected = await transport.choose(action === 'export');
	if (!selected) return { kind: 'noop' };
	if (action === 'export') {
		await transport.export(selected);
		return { kind: 'exported' };
	}
	const value = await transport.preview(selected);
	const targets = await transport.destinations();
	return { kind: 'preview', value, targets, path: selected };
}
