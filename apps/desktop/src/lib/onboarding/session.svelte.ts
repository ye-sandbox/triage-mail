import { transfer } from '../backup/transport';
import { nativeAvailable } from '$lib/inbox/transport';
import { toShellAccounts } from '$lib/inbox/workspace.svelte';
import { appStore } from '$lib/stores/app.svelte';
import { AccountController } from './controller.svelte';
import { nativeAccountTransport } from './transport';
import type { AccountTransport, RecoveryTransport, VaultStatus } from './types';

export interface OnboardingSessionOptions {
	accountTransport?: AccountTransport;
	recoveryTransport?: RecoveryTransport;
	available?: () => boolean;
}

export class OnboardingSession {
	readonly accounts: AccountController;
	readonly recoveryTransport: RecoveryTransport;
	readonly available: () => boolean;

	recovery = $state(false);
	vaultStatus = $state<VaultStatus | null>(null);
	booting = $state(false);
	booted = $state(false);
	private revision = 0;

	constructor(options: OnboardingSessionOptions = {}) {
		const accountTransport = options.accountTransport ?? nativeAccountTransport;
		this.accounts = new AccountController(accountTransport);
		this.recoveryTransport = options.recoveryTransport ?? transfer;
		this.available = options.available ?? nativeAvailable;
	}

	get isNative(): boolean {
		return this.available();
	}

	get needsOnboarding(): boolean {
		if (!this.available()) return false;
		if (this.recovery) return true;
		if (this.accounts.phase === 'locked' || this.accounts.phase === 'error') return true;
		if (this.accounts.phase === 'ready' && this.accounts.accounts.length === 0) return true;
		return false;
	}

	syncShellState() {
		if (!this.available()) return;
		appStore.vaultUnlocked = !this.recovery && this.accounts.phase === 'ready';
		if (this.accounts.phase === 'ready') {
			appStore.accounts = toShellAccounts(this.accounts.accounts);
		}
	}

	private async loadVaultStatus(revision: number): Promise<void> {
		if (!this.available() || !this.accounts.transport.vaultStatus) return;
		try {
			const status = await this.accounts.transport.vaultStatus();
			if (revision !== this.revision) return;
			this.vaultStatus = status;
		} catch {
			// Older native hosts or partial mocks retain default unlock presentation.
		}
	}

	async boot(force = false): Promise<void> {
		if (this.booted && !force) return;
		const revision = ++this.revision;
		this.booting = true;

		try {
			if (!this.available()) {
				this.recovery = false;
				this.vaultStatus = null;
				await this.accounts.load(false);
				if (revision !== this.revision) return;
				this.booted = true;
				return;
			}

			if (this.accounts.transport === nativeAccountTransport || this.recoveryTransport !== transfer) {
				try {
					const hasRecovery = await this.recoveryTransport.available();
					if (revision !== this.revision) return;
					if (hasRecovery) {
						this.recovery = true;
						this.booted = true;
						this.syncShellState();
						return;
					}
				} catch {
					// Older native hosts retain normal startup.
				}
			}

			this.recovery = false;
			await this.loadVaultStatus(revision);
			if (revision !== this.revision) return;
			await this.accounts.load(true);
			if (revision !== this.revision) return;
			this.booted = true;
			this.syncShellState();
		} finally {
			if (revision === this.revision) {
				this.booting = false;
			}
		}
	}

	async continueWithoutRecovery(): Promise<void> {
		const revision = ++this.revision;
		this.recovery = false;
		await this.loadVaultStatus(revision);
		if (revision !== this.revision) return;
		await this.accounts.load(this.available());
		if (revision !== this.revision) return;
		this.booted = true;
		this.syncShellState();
	}

	async completeRecovery(): Promise<void> {
		const revision = ++this.revision;
		this.recovery = false;
		await this.loadVaultStatus(revision);
		if (revision !== this.revision) return;
		await this.accounts.load(this.available());
		if (revision !== this.revision) return;
		this.booted = true;
		this.syncShellState();
	}

	dispose() {
		this.revision++;
		this.accounts.dispose();
	}
}

export const onboardingSession = new OnboardingSession();
