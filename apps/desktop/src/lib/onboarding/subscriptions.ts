export type PlanBadge = 'Ultra' | 'Pro' | 'One' | 'Workspace' | 'Plus';
export type BadgeOverride =
	| { mode: 'Auto' }
	| { mode: 'Hidden' }
	| { mode: 'Preset'; value: PlanBadge }
	| { mode: 'Custom'; value: string };
export type StorageQuota = { used_bytes: number; limit_bytes: number };
export type AccountSubscription = {
	account_id: string;
	effective_badge: string | null;
	badge_override: BadgeOverride;
	inferred_badge: PlanBadge | null;
	quota: StorageQuota | null;
};

export const MAX_CUSTOM_BADGE_CHARS = 12;
export const PLAN_BADGES: readonly PlanBadge[] = ['Ultra', 'Pro', 'One', 'Workspace', 'Plus'];

/** Select option values: `auto`, `hidden`, `custom` or `preset:<Badge>`. */
export type BadgeSelectValue = string;

export function overrideToSelect(override: BadgeOverride): BadgeSelectValue {
	switch (override.mode) {
		case 'Preset':
			return `preset:${override.value}`;
		case 'Custom':
			return 'custom';
		case 'Hidden':
			return 'hidden';
		default:
			return 'auto';
	}
}

export function selectToOverride(value: BadgeSelectValue, customLabel: string): BadgeOverride | null {
	if (value === 'auto') return { mode: 'Auto' };
	if (value === 'hidden') return { mode: 'Hidden' };
	if (value === 'custom') {
		const label = customLabel.trim();
		if (label.length === 0 || [...label].length > MAX_CUSTOM_BADGE_CHARS) return null;
		return { mode: 'Custom', value: label };
	}
	const badge = PLAN_BADGES.find((candidate) => value === `preset:${candidate}`);
	return badge ? { mode: 'Preset', value: badge } : null;
}

export function badgeTone(label: string): string {
	switch (label) {
		case 'ULTRA':
			return 'border-status-warning text-status-warning';
		case 'PRO':
			return 'border-accent text-accent';
		case 'ONE':
			return 'border-status-healthy text-status-healthy';
		default:
			return 'border-separator text-foreground';
	}
}

export function formatBytes(bytes: number): string {
	const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
	let value = bytes;
	let unit = 0;
	while (value >= 1024 && unit < units.length - 1) {
		value /= 1024;
		unit += 1;
	}
	return `${unit === 0 ? value : value.toFixed(value >= 100 ? 0 : 1)} ${units[unit]}`;
}

export function quotaPercent(quota: StorageQuota): number {
	if (quota.limit_bytes <= 0) return 0;
	return Math.min(100, Math.max(0, Math.round((quota.used_bytes / quota.limit_bytes) * 100)));
}

export function quotaSummary(quota: StorageQuota): string {
	return `${formatBytes(quota.used_bytes)} / ${formatBytes(quota.limit_bytes)}`;
}
