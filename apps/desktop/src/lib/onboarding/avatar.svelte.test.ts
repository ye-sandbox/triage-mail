import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { AvatarController, avatarFallback, MAX_AVATAR_BYTES } from './avatar-controller.svelte';
import type { AccountAvatar, AvatarTransport } from './avatar-types';
import AccountPhoto from './AccountPhoto.svelte';
import { i18n } from '../i18n/index.svelte';

const photo: AccountAvatar = {
	bytes: [137, 80, 78, 71],
	mime_type: 'image/png',
	width: 128,
	height: 128
};
function fixture() {
	const transport = {
		get: vi.fn<AvatarTransport['get']>().mockResolvedValue(null),
		set: vi.fn<AvatarTransport['set']>().mockResolvedValue(photo),
		remove: vi.fn<AvatarTransport['remove']>().mockResolvedValue()
	};
	return { transport, controller: new AvatarController('a', transport) };
}
function file(size = 3): File {
	const value = new File([new Uint8Array(size)], 'private-path.png', {
		type: 'image/png'
	});
	Object.defineProperty(value, 'arrayBuffer', {
		configurable: true,
		value: async () => new Uint8Array(size).buffer
	});
	return value;
}
async function settle() {
	for (let i = 0; i < 6; i++) {
		await tick();
		flushSync();
	}
}
afterEach(() => {
	document.body.replaceChildren();
	vi.restoreAllMocks();
	i18n.setLocale('en');
});

it('sends only bytes and account identity, preserves the prior image on failure, and removes it', async () => {
	const { controller, transport } = fixture();
	await controller.load();
	await controller.replace(file());
	expect(transport.set).toHaveBeenCalledWith('a', [0, 0, 0]);
	expect(controller.src).toMatch(/^data:image\/png;base64,/);
	const previous = controller.src;
	transport.set.mockRejectedValueOnce('private-path-and-error');
	await controller.replace(file());
	expect(controller.src).toBe(previous);
	expect(controller.error).toBe('saveError');
	await controller.remove();
	expect(transport.remove).toHaveBeenCalledWith('a');
	expect(controller.src).toBe('');
});

it('rejects oversized or empty files before reading them and ignores operations after disposal', async () => {
	const { controller, transport } = fixture();
	await controller.load();
	for (const size of [0, MAX_AVATAR_BYTES + 1]) {
		const original = file(size);
		const read = vi.spyOn(original, 'arrayBuffer');
		await controller.replace(original);
		expect(read).not.toHaveBeenCalled();
		expect(controller.error).toBe('invalidFile');
	}
	expect(transport.set).not.toHaveBeenCalled();
	controller.dispose();
	await controller.replace(file());
	expect(transport.set).not.toHaveBeenCalled();
});

it('blocks duplicate operations and late results cannot revive disposed photos', async () => {
	const { controller, transport } = fixture();
	await controller.load();
	let resolve!: (avatar: AccountAvatar) => void;
	transport.set.mockImplementationOnce(
		() =>
			new Promise((done) => {
				resolve = done;
			})
	);
	const pending = controller.replace(file());
	await Promise.resolve();
	await Promise.resolve();
	await controller.replace(file());
	expect(transport.set).toHaveBeenCalledTimes(1);
	controller.dispose();
	resolve(photo);
	await pending;
	expect(controller.src).toBe('');
});

it('uses safe fallback when loading fails and derives stable initials and accents', async () => {
	const { controller, transport } = fixture();
	transport.get.mockRejectedValueOnce('VaultUnavailable');
	await controller.load();
	expect(controller.loading).toBe(false);
	expect(controller.error).toBe('loadError');
	expect(controller.src).toBe('');
	expect(avatarFallback('a', 'reader.name@example.com')).toEqual(
		avatarFallback('a', 'reader.name@example.com')
	);
	expect(avatarFallback('a', 'reader.name@example.com').initials).toBe('RN');
});

it('renders bilingual controls and preserves the preview after a failed removal', async () => {
	const { transport } = fixture();
	transport.get.mockResolvedValue(photo);
	const target = document.createElement('div');
	document.body.append(target);
	const component = mount(AccountPhoto, {
		target,
		props: { accountId: 'a', label: 'reader@example.com', transport }
	});
	await settle();
	expect(target.querySelector('input')?.accept).toBe('image/jpeg,image/png,image/webp');
	expect(target.querySelector('img')?.src).toMatch(/^data:image\/png;base64,/);
	transport.remove.mockRejectedValueOnce('private-diagnostic');
	target.querySelector('button')?.click();
	await settle();
	expect(target.querySelector('img')).not.toBeNull();
	expect(target.querySelector('[role="alert"]')?.textContent).toContain('could not be saved');
	expect(target.textContent).not.toContain('private-diagnostic');
	i18n.setLocale('pt-BR');
	await settle();
	expect(target.textContent).toContain('Remover foto');
	await unmount(component);
});

it('handles file selection through the local input and reloads persisted thumbnails on remount', async () => {
	const { transport } = fixture();
	const target = document.createElement('div');
	document.body.append(target);
	let component = mount(AccountPhoto, {
		target,
		props: { accountId: 'a', label: 'reader@example.com', transport }
	});
	await settle();
	const input = target.querySelector<HTMLInputElement>('input')!;
	Object.defineProperty(input, 'files', { value: [file()], configurable: true });
	input.dispatchEvent(new Event('change', { bubbles: true }));
	await settle();
	expect(transport.set).toHaveBeenCalledWith('a', [0, 0, 0]);
	expect(input.value).toBe('');
	expect(target.querySelector('img')).not.toBeNull();
	await unmount(component);
	transport.get.mockResolvedValue(photo);
	component = mount(AccountPhoto, {
		target,
		props: { accountId: 'a', label: 'reader@example.com', transport }
	});
	await settle();
	expect(target.querySelector('img')?.src).toMatch(/^data:image\/png;base64,/);
	await unmount(component);
});
