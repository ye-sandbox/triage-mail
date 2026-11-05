<script lang="ts">
	import { untrack } from 'svelte';
	import { transferGuards } from './guards';
	import { SettingsFormGuard } from '../settings/guards';
	import type { AccountConfig } from '../onboarding/types';
	import BackupForm from './BackupForm.svelte';
	import PortableForm from './PortableForm.svelte';
	import { labels } from './labels';
	import { i18n } from '../i18n/index.svelte';
	let { guard, accounts }: { guard: SettingsFormGuard; accounts: AccountConfig[] } = $props();
	const forms = transferGuards(untrack(() => guard));
	let busy = $state(false);
</script>

<section class="space-y-8">
	<h2 class="text-xl font-semibold">{labels[i18n.locale].title}</h2>
	<fieldset disabled={busy}>
		<BackupForm
			guard={forms.backup}
			onbusy={(value) => {
				busy = value;
			}}
		/>
	</fieldset>
	<hr class="border-[var(--separator)]" />
	<fieldset disabled={busy}>
		<PortableForm
			guard={forms.portable}
			{accounts}
			onbusy={(value) => {
				busy = value;
			}}
		/>
	</fieldset>
</section>
