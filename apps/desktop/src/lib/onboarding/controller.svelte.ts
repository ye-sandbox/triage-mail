import { i18n } from '../i18n/index.svelte';
import type {
	AccountConfig,
	AccountSubmission,
	AccountTransport,
	ConnectionDiagnosticCode,
	ConnectionTestResult,
	LocalArchiveFormat,
	LocalArchiveImportResult,
	PasswordCredentials,
	Reauthorization
} from './types';

const DIAGNOSTIC_CODES: ReadonlySet<ConnectionDiagnosticCode> = new Set([
	'TlsHandshakeFailed',
	'AuthenticationRejected',
	'ImapDisabledHint',
	'OAuthAccessDenied',
	'OAuthInvalidClient',
	'OAuthCancelled',
	'NetworkTimeout'
]);

export function classifyDiagnosticError(err: unknown): ConnectionDiagnosticCode | null {
	if (typeof err === 'string' && DIAGNOSTIC_CODES.has(err as ConnectionDiagnosticCode)) {
		return err as ConnectionDiagnosticCode;
	}
	if (err && typeof err === 'object') {
		for (const code of DIAGNOSTIC_CODES) {
			if (code in err) return code;
		}
	}
	return null;
}

export class AccountController {
	phase = $state<'loading' | 'locked' | 'ready' | 'error' | 'browser'>('loading');
	accounts = $state<AccountConfig[]>([]);
	busy = $state(false);
	testing = $state(false);
	authorizing = $state(false);
	reauthorizingAccountId = $state<string | null>(null);
	testResult = $state<ConnectionTestResult | null>(null);
	diagnostic = $state<ConnectionDiagnosticCode | null>(null);
	accountDiagnostics = $state<Record<string, ConnectionDiagnosticCode>>({});
	importProgress = $state<{ importedCount: number; active: boolean }>({
		importedCount: 0,
		active: false
	});
	error = $state('');
	private active = true;
	private revision = 0;
	constructor(readonly transport: AccountTransport) {}
	clearDiagnostics() {
		this.diagnostic = null;
		this.testResult = null;
		this.error = '';
	}
	async loadDiagnostics() {
		if (!this.active || !this.transport.diagnostics) return;
		const revision = this.revision;
		try {
			const diagnostics = await this.transport.diagnostics();
			if (!this.active || revision !== this.revision) return;
			this.accountDiagnostics = diagnostics;
		} catch {
			// Best-effort diagnostics polling; never disrupt account directory loading.
		}
	}
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
			if (this.transport.diagnostics) {
				void this.loadDiagnostics();
			}
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
	async testConnection(
		config: AccountConfig,
		credentials: PasswordCredentials
	): Promise<boolean> {
		if (this.busy || this.testing || !this.active || !this.transport.testConnection) {
			return false;
		}
		this.testing = true;
		this.error = '';
		this.diagnostic = null;
		this.testResult = null;
		let ok = false;
		try {
			// INVARIANT: Credentials go straight to transient IPC and never touch reactive state.
			const pending = this.transport.testConnection(config, credentials);
			credentials = { AppPassword: { secret: '' } };
			const result = await pending;
			if (!this.active) return false;
			this.testResult = result;
			ok = result.ok;
			if (!result.ok) {
				this.diagnostic = result.diagnostic ?? null;
				if (!result.diagnostic) {
					this.error = i18n.t.accountForm.testLoginFailed;
				}
			}
		} catch (err: unknown) {
			if (this.active) {
				const code = classifyDiagnosticError(err);
				this.diagnostic = code;
				this.testResult = { ok: false, ...(code ? { diagnostic: code } : {}) };
				if (!code) {
					this.error = i18n.t.accountForm.testLoginFailed;
				}
			}
		} finally {
			if (this.active) this.testing = false;
		}
		return ok;
	}
	async add(config: AccountConfig, submission: AccountSubmission) {
		if (this.busy || this.testing || !this.active) return false;
		const oauth = 'authorization' in submission;
		this.busy = true;
		this.authorizing = false;
		this.error = '';
		this.diagnostic = null;
		this.testResult = null;
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
		} catch (err: unknown) {
			if (this.active) {
				const code = classifyDiagnosticError(err);
				this.diagnostic = code;
				this.error = code
					? ''
					: oauth
						? 'Authorization did not complete. Check your provider settings or try again.'
						: 'Account could not be saved. Check your settings and try again.';
			}
		} finally {
			if (this.active) {
				this.authorizing = false;
				// Reconcile successful registration racing an explicit cancellation.
				const error = this.error;
				const diagnostic = this.diagnostic;
				await this.load();
				if (this.active) {
					if (this.phase === 'ready') {
						this.error = error;
						this.diagnostic = diagnostic;
					}
					this.busy = false;
				}
			}
		}
		return success;
	}
	async reauthorize(input: Reauthorization): Promise<boolean> {
		if (this.busy || this.testing || !this.active || !this.transport.reauthorize) return false;
		const accountId = input.account_id;
		this.busy = true;
		this.authorizing = false;
		this.reauthorizingAccountId = accountId;
		this.error = '';
		this.diagnostic = null;
		let success = false;
		try {
			// INVARIANT: Client override material goes straight to IPC and is cleared immediately.
			const pending = this.transport.reauthorize(input, () => {
				if (this.active) this.authorizing = true;
				else void this.transport.cancel().catch(() => {});
			});
			input = { account_id: '', authorization_endpoint: '', scopes: '' };
			await pending;
			success = true;
			if (this.active) {
				const { [accountId]: _cleared, ...rest } = this.accountDiagnostics;
				this.accountDiagnostics = rest;
			}
		} catch (err: unknown) {
			if (this.active) {
				const code = classifyDiagnosticError(err);
				this.diagnostic = code;
				this.error = code ? '' : i18n.t.onboarding.reauthorizeError;
			}
		} finally {
			if (this.active) {
				this.authorizing = false;
				this.reauthorizingAccountId = null;
				const error = this.error;
				const diagnostic = this.diagnostic;
				await this.load();
				if (this.active) {
					if (this.phase === 'ready') {
						this.error = error;
						this.diagnostic = diagnostic;
					}
					this.busy = false;
				}
			}
		}
		return success;
	}
	async importArchive(
		path: string,
		emailDisplay: string,
		format: LocalArchiveFormat
	): Promise<LocalArchiveImportResult | null> {
		if (this.busy || this.testing || !this.active || !this.transport.importLocalArchive) return null;
		this.busy = true;
		this.error = '';
		this.diagnostic = null;
		this.importProgress = { importedCount: 0, active: true };
		let result: LocalArchiveImportResult | null = null;
		try {
			result = await this.transport.importLocalArchive(path, emailDisplay, format, (event) => {
				if (this.active && event.new_messages_count > 0) {
					this.importProgress = {
						importedCount: this.importProgress.importedCount + event.new_messages_count,
						active: true
					};
				}
			});
		} catch (err: unknown) {
			if (this.active) {
				this.error =
					err === 'AccountExists'
						? i18n.t.accountForm.localArchiveConflictError
						: i18n.t.accountForm.localArchiveImportError;
			}
		} finally {
			if (this.active) {
				const error = this.error;
				await this.load();
				if (this.active) {
					if (this.phase === 'ready') this.error = error;
					this.importProgress = { importedCount: 0, active: false };
					this.busy = false;
				}
			}
		}
		return result;
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
