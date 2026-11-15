import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { i18n } from '$lib/i18n/index.svelte';
import WdgAutomationPage from './WdgAutomationPage.svelte';
import { createDemoAutomationBackend } from './automation.fixtures';
import { automationLabels } from './labels';
import { simulateRulesLocally, validateWebhookOrigin } from './simulator';

let cleanup: (() => void) | undefined;

afterEach(() => {
	cleanup?.();
	cleanup = undefined;
	document.body.innerHTML = '';
	i18n.locale = 'en';
});

async function settle() {
	for (let i = 0; i < 20; i++) await Promise.resolve();
	flushSync();
}

describe('automation labels parity', () => {
	it('maintains 1:1 key parity between en and pt-BR dictionaries', () => {
		const collectKeys = (obj: Record<string, unknown>, prefix = ''): string[] => {
			const keys: string[] = [];
			for (const [k, v] of Object.entries(obj)) {
				const path = prefix ? `${prefix}.${k}` : k;
				if (v && typeof v === 'object' && !Array.isArray(v)) {
					keys.push(...collectKeys(v as Record<string, unknown>, path));
				} else {
					keys.push(path);
				}
			}
			return keys.sort();
		};
		expect(collectKeys(automationLabels.en as unknown as Record<string, unknown>)).toEqual(
			collectKeys(automationLabels['pt-BR'] as unknown as Record<string, unknown>)
		);
	});
});

describe('simulator helpers', () => {
	it('blocks localhost, IP literals, and non-443 ports in validateWebhookOrigin', () => {
		expect(validateWebhookOrigin('http://hooks.example.org').error).toBeDefined();
		expect(validateWebhookOrigin('https://localhost:443/hook').error).toBeDefined();
		expect(validateWebhookOrigin('https://127.0.0.1/hook').error).toBeDefined();
		expect(validateWebhookOrigin('https://hooks.example.org:8443/hook').error).toBeDefined();
		expect(validateWebhookOrigin('https://hooks.ledger.example.org/ingest')).toEqual({
			origin: 'https://hooks.ledger.example.org'
		});
	});

	it('evaluates Matched, Excluded, Disabled, and Stopped precedence', () => {
		const backend = createDemoAutomationBackend();
		const results = simulateRulesLocally(
			[
				{
					version: 1,
					id: 'r1',
					revision: 1,
					name: 'Stop rule',
					enabled: true,
					position: 0,
					all: [{ Account: 'acct_personal_02' }],
					any: [{ Subject: 'Fatura' }],
					except: [{ Domain: 'promo.example.org' }],
					actions: [{ Tag: 'Payable' }],
					stop_processing: true
				},
				{
					version: 1,
					id: 'r2',
					revision: 1,
					name: 'Second rule',
					enabled: true,
					position: 1,
					all: [{ Account: 'acct_personal_02' }],
					any: [],
					except: [],
					actions: ['Star'],
					stop_processing: false
				}
			],
			undefined,
			backend.sampleMessages.filter(
				(m) => m.id === 'msg_02_boleto' || m.id === 'msg_05_promo_bill'
			)
		);

		const boletoPlan = results.find(([id]) => id === 'msg_02_boleto')![1];
		expect(boletoPlan.actions).toEqual([{ Tag: 'Payable' }]);
		expect(boletoPlan.explanations[0].outcome).toBe('Matched');
		expect(boletoPlan.explanations[1].outcome).toBe('Stopped');

		const promoPlan = results.find(([id]) => id === 'msg_05_promo_bill')![1];
		expect(promoPlan.explanations[0].outcome).toBe('Excluded');
		expect(promoPlan.explanations[1].outcome).toBe('Matched');
	});
});

describe('WdgAutomationPage (UI v2 workspace)', () => {
	it('loads demo workspace, navigates across all 3 governance tabs, and switches locale', async () => {
		const demo = createDemoAutomationBackend();
		const target = document.createElement('div');
		document.body.appendChild(target);
		const component = mount(WdgAutomationPage, {
			target,
			props: {
				rulesTransport: demo.rulesTransport,
				alertsTransport: demo.alertsTransport,
				scheduleTransport: demo.scheduleTransport,
				reviewTransport: demo.reviewTransport,
				categoriesTransport: demo.categoriesTransport,
				accounts: demo.accounts,
				selectedIds: demo.sampleMessages.map((m) => m.id),
				ruleMeta: demo.ruleMeta,
				historyMeta: demo.historyMeta,
				senderMeta: demo.senderMeta,
				overrideMeta: demo.overrideMeta
			}
		});
		cleanup = () => unmount(component);
		await settle();

		expect(target.textContent).toContain('Automation & Triage Governance');
		expect(target.textContent).toContain('2/3 rules active');
		expect(target.textContent).toContain('3 destinations');
		expect(target.textContent).toContain('FEBRABAN Boletos & Utility Invoices');

		const alertsTab = [...target.querySelectorAll<HTMLButtonElement>('button[role="tab"]')].find(
			(b) => b.textContent?.includes('Alert Destinations')
		)!;
		alertsTab.click();
		await settle();
		expect(target.textContent).toContain('Native OS Notification');
		expect(target.textContent).toContain('Delivery History');

		const govTab = [...target.querySelectorAll<HTMLButtonElement>('button[role="tab"]')].find(
			(b) => b.textContent?.includes('Senders & Overrides')
		)!;
		govTab.click();
		await settle();
		expect(target.textContent).toContain('New Senders Screener Queue');
		expect(target.textContent).toContain('Local Category Overrides');
		expect(target.textContent).toContain('billing@energia.example.org');

		i18n.setLocale('pt-BR');
		await settle();
		expect(target.textContent).toContain('Automação e Governança de Triagem');
		expect(target.textContent).toContain('Substituições Locais de Categoria');
	});
});
