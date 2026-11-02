import type { DiscoveryTransport, DiscoveredProvider } from './discovery';
import type { AuthMethod } from './types';
import type { ProviderPreset } from './presets';

export interface ConnectionSettings {
	host: string;
	port: number;
	method: AuthMethod;
	authorize: string;
	token: string;
	scopes: string;
}
export class DiscoveryController {
	settings = $state<ConnectionSettings>({
		host: 'imap.gmail.com',
		port: 993,
		method: 'AppPassword',
		authorize: '',
		token: '',
		scopes: ''
	});
	selectedPreset = $state('Gmail');
	busy = $state(false);
	status = $state<
		'idle' | 'found' | 'unresolved' | 'invalid' | 'unavailable' | 'probeSuccess' | 'probeFailed'
	>('idle');
	provider = $state<DiscoveredProvider | null>(null);
	private active = true;
	private revision = 0;
	private manual = false;
	private timer: ReturnType<typeof setTimeout> | undefined;
	constructor(
		private readonly transport: DiscoveryTransport,
		private readonly beforeReplace: () => void = () => {}
	) {}

	invalidate() {
		this.revision++;
		clearTimeout(this.timer);
		this.busy = false;
		this.provider = null;
		this.status = 'idle';
	}
	protectCredentials() {
		// INVARIANT: A late suggestion must not change the identity/mode after credential entry.
		this.revision++;
		clearTimeout(this.timer);
		this.busy = false;
		this.manual = true;
	}
	edit() {
		this.invalidate();
		this.manual = true;
		this.selectedPreset = '';
	}
	preset(preset: ProviderPreset) {
		this.edit();
		this.selectedPreset = preset.name;
		this.settings = {
			host: preset.host,
			port: preset.port,
			method: preset.method,
			authorize: preset.authEndpoint ?? '',
			token: preset.tokenEndpoint ?? '',
			scopes: preset.scopes ?? ''
		};
	}
	input() {
		this.invalidate();
	}
	blur(email: string) {
		clearTimeout(this.timer);
		if (!this.manual && email.trim())
			this.timer = setTimeout(() => void this.lookup(email, false), 200);
	}
	async lookup(email: string, online: boolean) {
		if (!this.active || this.manual) return;
		this.invalidate();
		const revision = this.revision;
		this.busy = true;
		try {
			const provider = await this.transport.discover(email, online);
			if (!this.active || revision !== this.revision) return;
			this.beforeReplace();
			this.provider = provider;
			this.status = provider.imap_host ? 'found' : 'unresolved';
			this.selectedPreset = provider.imap_host ? provider.provider_name : '';
			this.settings = {
				host: provider.imap_host ?? '',
				port: provider.imap_port,
				method: provider.recommended_auth,
				authorize: provider.oauth_endpoints?.authorization_endpoint ?? '',
				token: provider.oauth_endpoints?.token_endpoint ?? '',
				scopes: provider.oauth_endpoints?.scopes ?? ''
			};
		} catch (error: unknown) {
			if (this.active && revision === this.revision)
				this.status = error === 'InvalidRequest' ? 'invalid' : 'unavailable';
		} finally {
			if (this.active && revision === this.revision) this.busy = false;
		}
	}
	async online(email: string) {
		// INVARIANT: Explicit lookup may replace manual settings; blur never does.
		this.manual = false;
		await this.lookup(email, true);
	}
	async probe() {
		if (
			!this.active ||
			this.busy ||
			this.settings.port !== 993 ||
			this.settings.method === 'ProtonBridgeLocal'
		)
			return;
		const revision = ++this.revision;
		clearTimeout(this.timer);
		const host = this.settings.host;
		this.busy = true;
		try {
			const ok = await this.transport.probe(host);
			if (this.active && revision === this.revision)
				this.status = ok ? 'probeSuccess' : 'probeFailed';
		} catch {
			if (this.active && revision === this.revision) this.status = 'probeFailed';
		} finally {
			if (this.active && revision === this.revision) this.busy = false;
		}
	}
	dispose() {
		this.active = false;
		this.invalidate();
	}
}
