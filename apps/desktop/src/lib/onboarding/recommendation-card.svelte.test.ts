import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import WdgMethodRecommendationCard from './WdgMethodRecommendationCard.svelte';

let app: ReturnType<typeof mount> | undefined;
afterEach(() => {
	if (app) unmount(app);
	app = undefined;
	document.body.innerHTML = '';
});

describe('WdgMethodRecommendationCard', () => {
	it('renders the recommended option and dispatches actions', () => {
		const onAction = vi.fn();
		const onOffline = vi.fn();
		app = mount(WdgMethodRecommendationCard, {
			target: document.body,
			props: { kind: 'google', method: 'AppPassword', onAction, onOffline }
		});
		flushSync();
		const buttons = [...document.querySelectorAll('button')];
		expect(buttons).toHaveLength(3);
		expect(buttons[0].getAttribute('aria-pressed')).toBe('true');
		expect(buttons[1].getAttribute('aria-pressed')).toBe('false');
		buttons[1].click();
		expect(onAction).toHaveBeenCalledWith({ type: 'preset', name: 'Gmail OAuth' });
		buttons[2].click();
		expect(onOffline).toHaveBeenCalledOnce();
	});
});
