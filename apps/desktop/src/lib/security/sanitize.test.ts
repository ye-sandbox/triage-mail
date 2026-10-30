import { describe, expect, it } from 'vitest';
import { sanitizeEmailHtml } from './sanitize';

describe('sanitizeEmailHtml', () => {
	it('strips script tags and inline event handlers while injecting normalized dark-mode CSS', () => {
		const maliciousHtml =
			'<div>Hello <script>alert(1)</script><a href="https://example.com" onclick="steal()">Link</a></div>';
		const output = sanitizeEmailHtml(maliciousHtml);

		expect(output).not.toContain('<script>');
		expect(output).not.toContain('onclick');
		expect(output).toContain('color-scheme: dark light;');
		expect(output).toContain('Hello');
	});

	it('blocks remote tracking pixel images by default and allows them only on explicit opt-in', () => {
		const htmlWithPixel =
			'<p>Invoice</p><img src="https://tracker.example.com/pixel.gif" width="1" height="1" />';

		const blocked = sanitizeEmailHtml(htmlWithPixel, { allowRemoteImages: false });
		expect(blocked).not.toContain('https://tracker.example.com/pixel.gif');
		expect(blocked).toContain('data-blocked-remote-src="true"');

		const allowed = sanitizeEmailHtml(htmlWithPixel, { allowRemoteImages: true });
		expect(allowed).toContain('https://tracker.example.com/pixel.gif');
	});
});
