<script lang="ts">
	import type { SettingsModuleProps } from '../types';

	let {
		settings,
		transport,
		busy = false,
		onReload
	}: Partial<SettingsModuleProps> = $props();
</script>

<div class="space-y-6" aria-label="Privacy settings">
	<div>
		<h2 class="text-lg font-semibold text-[var(--text)]">Privacy &amp; Security</h2>
		<p class="text-sm text-[var(--secondary)]">
			Zero-knowledge principles and local security guarantees.
		</p>
	</div>

	<section class="space-y-2 rounded-lg border border-[var(--separator)] bg-[var(--surface)] p-4">
		<h3 class="text-base font-medium text-[var(--text)]">Remote Images &amp; Tracking Pixels</h3>
		<p class="text-sm text-[var(--secondary)]">
			Remote images remain blocked by default to prevent tracking pixels from reporting your IP
			address, location, or read status to third-party senders.
		</p>
		<p class="text-sm text-[var(--secondary)]">
			You can allow HTTPS images for a single open message in the reader. Changing messages resets
			consent immediately.
		</p>
	</section>

	<section class="space-y-2 rounded-lg border border-[var(--separator)] bg-[var(--surface)] p-4">
		<h3 class="text-base font-medium text-[var(--text)]">Zero Telemetry &amp; Network Isolation</h3>
		<p class="text-sm text-[var(--secondary)]">
			triage-mail contains zero telemetry, zero analytics trackers, and zero background network calls.
			Outbound connections occur strictly to your configured IMAP servers, explicit RFC 8058
			one-click unsubscribe requests when you click them, or explicitly enabled AI endpoints.
		</p>
	</section>

	<section class="space-y-2 rounded-lg border border-[var(--separator)] bg-[var(--surface)] p-4">
		<h3 class="text-base font-medium text-[var(--text)]">Local Vault &amp; Database Encryption</h3>
		<p class="text-sm text-[var(--secondary)]">
			Credentials and OAuth tokens are stored in the OS Keyring via Argon2id encryption. Email
			metadata, bodies, and full-text search indexes are stored locally in SQLCipher-encrypted
			SQLite.
		</p>
	</section>
</div>
