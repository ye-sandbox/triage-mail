import type { AccountConfig, AccountTransport, AccountSubmission } from './types';

export class AccountController {
	phase = $state<'loading' | 'locked' | 'ready' | 'error' | 'browser'>('loading');
	accounts = $state<AccountConfig[]>([]);
	busy = $state(false);
	authorizing = $state(false);
	error = $state('');
	private active = true;
	private revision = 0;
	constructor(readonly transport: AccountTransport) {}
	async load(native = true) {
		if (!this.active) return;
		const revision = ++this.revision;
		if (!native) {
			this.phase = 'browser';
			return;
		}
		this.error = '';
		try {
			const accounts = await this.transport.list();
			if (!this.active || revision !== this.revision) return;
			this.accounts = accounts;
			this.phase = 'ready';
		} catch (error: unknown) {
			if (!this.active || revision !== this.revision) return;
			this.phase = error === 'VaultUnavailable' ? 'locked' : 'error';
			this.error = error === 'VaultUnavailable' ? '' : 'Local mail could not be opened. Try again.';
		}
	}
	async unlock(passphrase: string) {
		if (this.busy || !this.active) return;
		this.busy = true;
		this.error = '';
		try {
			const pending = this.transport.unlock(passphrase);
			passphrase = '';
			await pending;
			if (this.active) await this.load();
		} catch {
			if (this.active)
				this.error = 'Vault could not be unlocked. Check your passphrase and try again.';
		} finally {
			if (this.active) this.busy = false;
		}
	}
	async add(config: AccountConfig, submission: AccountSubmission) {
		if (this.busy || !this.active) return false;
		const oauth = 'authorization' in submission;
		this.busy = true;
		this.authorizing = false;
		this.error = '';
		let success = false;
		try {
			// INVARIANT: Credentials are passed straight to IPC; never retained in reactive state.
			const pending =
				'authorization' in submission
					? this.transport.authorize(config, submission.authorization, () => {
							if (this.active) this.authorizing = true;
							else void this.transport.cancel().catch(() => {});
						})
					: this.transport.add(config, submission.credentials);
			submission = { credentials: { AppPassword: { secret: '' } } };
			await pending;
			success = true;
		} catch {
			if (this.active)
				this.error = oauth
					? 'Authorization did not complete. Check your provider settings or try again.'
					: 'Account could not be saved. Check your settings and try again.';
		} finally {
			if (this.active) {
				this.authorizing = false;
				// Reconcile successful registration racing an explicit cancellation.
				const error = this.error;
				await this.load();
				if (this.active) {
					if (this.phase === 'ready') this.error = error;
					this.busy = false;
				}
			}
		}
		return success;
	}
	async cancel() {
		if (!this.authorizing) return;
		try {
			await this.transport.cancel();
		} catch {
			if (this.active) this.error = 'Authorization could not be cancelled. Try again.';
		}
	}
	dispose() {
		this.active = false;
		if (this.authorizing) void this.transport.cancel().catch(() => {});
	}
}
