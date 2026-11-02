<script lang="ts">
	import { untrack } from 'svelte';
	import type { InboxController } from './controller.svelte';
	import type { Mailboxes } from './mailboxes.svelte';
	import type { AccountConfig } from '../onboarding/types';
	let {
		inbox,
		mailboxes,
		accounts,
		connected
	}: {
		inbox: InboxController;
		mailboxes: Mailboxes;
		accounts: AccountConfig[];
		connected: boolean;
	} = $props();

	$effect(() => {
		const list = accounts;
		untrack(() => {
			mailboxes.reconcile(list);
			const scope = inbox.scope;
			if (
				scope &&
				scope !== 'GlobalAllMail' &&
				!list.some((a) => a.account_id === scope.AccountView.account_id)
			)
				void inbox.selectScope();
		});
	});
	$effect(() => () => mailboxes.dispose());
	$effect(() => {
		const revision = inbox.badgeRevision;
		if (revision)
			untrack(() => {
				for (const account of accounts) void mailboxes.refresh(account.account_id);
			});
	});
	$effect(() => {
		const deadlines = inbox.rows
			.map((r) => r.snooze_until_epoch_ms ?? 0)
			.filter((n) => n > Date.now());
		if (!deadlines.length || !connected) return;
		const timer = setTimeout(
			() => void inbox.refresh(),
			Math.min(2147483647, deadlines.reduce((a, b) => Math.min(a, b)) - Date.now() + 1)
		);
		return () => clearTimeout(timer);
	});
</script>

<svelte:window
	onfocus={() => {
		if (connected) void inbox.refresh();
	}}
/>
