import type { EmbeddingState } from './types';
import {
	PORTABLE_IMPORT_PREVIEW_FIXTURE,
	type PortableDefinitionDiffItem,
	type PortableImportPreview
} from './settings.fixtures';

export function getEmbeddingProgressPct(state: EmbeddingState): number {
	const total = state.indexed + state.pending + state.failed;
	return total > 0 ? Math.min(100, Math.round((state.indexed / total) * 100)) : 0;
}

export function getEmbeddingStatusTone(status: EmbeddingState['status']): string {
	switch (status) {
		case 'Ready':
			return 'border-status-healthy/40 bg-status-healthy/10 text-status-healthy';
		case 'Rebuilding':
			return 'border-status-info/40 bg-status-info/10 text-status-info';
		case 'MissingModelOrRuntime':
			return 'border-status-warning/40 bg-status-warning/10 text-status-warning';
		case 'Failed':
			return 'border-status-danger/40 bg-status-danger/10 text-status-danger';
		case 'Disabled':
		case 'Unavailable':
			return 'border-separator bg-field text-secondary';
	}
}

export function triggerBlobDownload(filename: string, content: string, mime: string) {
	const blob = new Blob([content], { type: mime });
	const url = URL.createObjectURL(blob);
	const anchor = document.createElement('a');
	anchor.href = url;
	anchor.download = filename;
	anchor.click();
	URL.revokeObjectURL(url);
}

export function buildImportPreviewFromJson(
	filename: string,
	rawText: string
): PortableImportPreview {
	const parsed = JSON.parse(rawText) as {
		rules?: { name: string }[];
		saved_views?: { name: string }[];
		views?: { name: string }[];
	};
	const savedViews = parsed.saved_views ?? parsed.views ?? [];
	const items: PortableDefinitionDiffItem[] = [
		...(parsed.rules ?? []).map((r, i) => ({
			id: `imp_r_${i}`,
			kind: 'rule' as const,
			name: r.name || `Rule ${i + 1}`,
			action: 'added' as const,
			summary: 'Imported deterministic rule'
		})),
		...savedViews.map((v, i) => ({
			id: `imp_v_${i}`,
			kind: 'saved_view' as const,
			name: v.name || `Saved View ${i + 1}`,
			action: 'added' as const,
			summary: 'Imported saved view'
		}))
	];

	return {
		source_filename: filename,
		schema_version: 'triage-portable-v1',
		exported_at_iso: new Date().toISOString(),
		counts: { added: items.length, updated: 0, conflicting: 0 },
		items: items.length > 0 ? items : structuredClone(PORTABLE_IMPORT_PREVIEW_FIXTURE.items)
	};
}
