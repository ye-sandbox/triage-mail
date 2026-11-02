import type { AccountAvatar, AvatarTransport } from './avatar-types';

export const MAX_AVATAR_BYTES = 2 * 1024 * 1024;
export type AvatarError = '' | 'loadError' | 'saveError' | 'invalidFile';

export class AvatarController {
	src = $state('');
	loading = $state(true);
	busy = $state(false);
	error = $state<AvatarError>('');
	private active = true;
	private revision = 0;
	constructor(
		readonly accountId: string,
		readonly transport: AvatarTransport
	) {}

	async load() {
		const revision = ++this.revision;
		try {
			const avatar = await this.transport.get(this.accountId);
			if (this.active && revision === this.revision) this.src = avatar ? source(avatar) : '';
		} catch {
			if (this.active && revision === this.revision) this.error = 'loadError';
		} finally {
			if (this.active && revision === this.revision) this.loading = false;
		}
	}

	async replace(file: File) {
		if (this.busy || !this.active || this.loading) return;
		if (!file.size || file.size > MAX_AVATAR_BYTES) {
			this.error = 'invalidFile';
			return;
		}
		await this.change(async () => {
			// INVARIANT: Never preview original bytes or send file paths/names across IPC.
			const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
			if (!this.active) return this.src;
			return source(await this.transport.set(this.accountId, bytes));
		});
	}

	async remove() {
		if (this.busy || !this.active || this.loading) return;
		await this.change(async () => {
			await this.transport.remove(this.accountId);
			return '';
		});
	}

	private async change(operation: () => Promise<string>) {
		this.busy = true;
		this.error = '';
		++this.revision;
		try {
			const src = await operation();
			if (this.active) this.src = src;
		} catch {
			if (this.active) this.error = 'saveError';
		} finally {
			if (this.active) this.busy = false;
		}
	}

	dispose() {
		this.active = false;
		++this.revision;
		this.src = '';
	}
}

function source(avatar: AccountAvatar): string {
	if (
		avatar.mime_type !== 'image/png' ||
		avatar.width !== 128 ||
		avatar.height !== 128 ||
		!avatar.bytes.length ||
		avatar.bytes.length > 128 * 1024 ||
		!avatar.bytes.every((byte) => Number.isInteger(byte) && byte >= 0 && byte <= 255)
	) {
		throw new Error('Invalid photo');
	}
	// INVARIANT: Only normalized native thumbnails are exposed to the image renderer.
	return `data:image/png;base64,${btoa(avatar.bytes.map((byte) => String.fromCharCode(byte)).join(''))}`;
}

export function avatarFallback(accountId: string, label: string) {
	let hash = 0;
	for (const char of accountId) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
	const words = label
		.split('@')[0]
		.split(/[\s._-]+/)
		.filter(Boolean);
	const initials = words
		.slice(0, 2)
		.map((word) => Array.from(word)[0])
		.join('')
		.toLocaleUpperCase();
	return { initials: initials || '?', color: hash % 3 };
}
