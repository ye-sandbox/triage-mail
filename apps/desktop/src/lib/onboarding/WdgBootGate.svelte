<script lang="ts">
	import { goto } from '$app/navigation';
	import { untrack } from 'svelte';
	import { i18n } from '../i18n/index.svelte';
	import { nativeTransport } from '../inbox/transport';
	import type { InboxTransport } from '../inbox/types';
	import { nativeRemindersTransport } from '../reminders/transport';
	import type { RemindersTransport } from '../reminders/types';
	import { nativeSettingsTransport } from '../settings/transport';
	import type { SettingsTransport } from '../settings/types';
	import InboxSession from './InboxSession.svelte';
	import { onboardingSession, type OnboardingSession } from './session.svelte';
	import WdgOnboardingPage from './WdgOnboardingPage.svelte';

	interface Props {
		session?: OnboardingSession;
		onNavigate?: (path: string) => void | Promise<void>;
		inboxTransport?: InboxTransport;
		settingsTransport?: SettingsTransport;
		remindersTransport?: RemindersTransport;
	}

	let {
		session = onboardingSession,
		onNavigate = (path: string) => goto(path, { replaceState: true }),
		inboxTransport = nativeTransport,
		settingsTransport = nativeSettingsTransport,
		remindersTransport = nativeRemindersTransport
	}: Props = $props();

	let redirected = false;

	$effect(() => {
		untrack(() => {
			void session.boot();
		});
	});

	$effect(() => {
		if (session.isNative && session.booted && !session.booting && session.needsOnboarding) {
			if (!redirected) {
				redirected = true;
				try {
					void Promise.resolve(onNavigate('/onboarding')).catch(() => {});
				} catch {
					// Unit tests mount WdgBootGate without booting SvelteKit's client router.
				}
			}
		} else {
			redirected = false;
		}
	});
</script>

{#if !session.isNative}
	<InboxSession connected={false} {inboxTransport} {settingsTransport} {remindersTransport} />
{:else if !session.booted || session.booting || (!session.recovery && session.accounts.phase === 'loading')}
	<main class="flex h-full min-h-0 flex-1 items-center justify-center bg-background p-8 text-foreground">
		<p role="status" class="text-sm text-secondary">{i18n.t.onboarding.openingMail}</p>
	</main>
{:else if session.needsOnboarding}
	<WdgOnboardingPage {session} />
{:else}
	<InboxSession
		connected={true}
		accountController={session.accounts}
		{inboxTransport}
		{settingsTransport}
		{remindersTransport}
	/>
{/if}
