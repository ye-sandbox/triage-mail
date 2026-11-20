import { expect, it } from 'vitest';
import fixture from '../../../../../crates/mail-engine/src/discovery/preset_domains.json';
import { providerPresets } from './presets';

// INVARIANT: The Rust offline preset table and these UI presets must agree on host/port/auth.
it('keeps every Rust offline preset domain consistent with the UI preset table', () => {
	expect(fixture.length).toBeGreaterThan(40);
	for (const entry of fixture) {
		const preset = providerPresets.find((p) => p.name === entry.name);
		expect(preset, entry.domain).toBeDefined();
		expect(preset?.host, entry.domain).toBe(entry.host);
		expect(preset?.port, entry.domain).toBe(entry.port);
		expect(preset?.method, entry.domain).toBe(entry.auth);
	}
});
