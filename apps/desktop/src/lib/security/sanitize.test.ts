import { describe, expect, it } from 'vitest';
import { hasBlockedRemoteImages, sanitizeEmailHtml } from './sanitize';

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

	it('applies explicit light and dark UI v2 reader stylesheets when requested', () => {
		const light = sanitizeEmailHtml('<p>Hello</p>', { theme: 'light' });
		expect(light).toContain('color-scheme: light;');
		expect(light).toContain('--reader-bg: #ffffff;');
		expect(light).toContain('--reader-scrollbar-thumb:');
		expect(light).toContain('::-webkit-scrollbar-thumb');

		const dark = sanitizeEmailHtml('<p>Hello</p>', { theme: 'dark' });
		expect(dark).toContain('color-scheme: dark;');
		expect(dark).toContain('--reader-bg: #121721;');
		expect(dark).toContain('--reader-scrollbar-thumb:');
		expect(dark).toContain('::-webkit-scrollbar-thumb');
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

it('detects blocked remote and lazy CID images without flagging safe inline raster images', () => {
	expect(hasBlockedRemoteImages('<p>Text only</p>')).toBe(false);
	expect(hasBlockedRemoteImages('<img src="data:image/png;base64,AAAA">')).toBe(false);
	expect(hasBlockedRemoteImages('<img src="https://cdn.example.com/chart.png">')).toBe(true);
	expect(hasBlockedRemoteImages('<img src="cid:inline-logo">')).toBe(true);
});

it('removes alternate network paths and navigation while placing CSP before sender markup', () => {
	const output = sanitizeEmailHtml(`<style>@import 'https://example.org/a';</style>
		<link rel="stylesheet" href="https://example.org/a"><meta http-equiv="refresh" content="0;url=https://example.org">
		<div style="background:url(https://example.org/a)" background="https://example.org/a">Text</div>
		<picture><source srcset="https://example.org/a"><img srcset="https://example.org/a" src=//example.org/a></picture>
		<svg><image href="https://example.org/a" /></svg><video poster="https://example.org/a"></video>
		<a href="https://example.org" ping="https://example.org/a">Link</a><form action="https://example.org"></form>`);
	expect(output).not.toContain('example.org');
	expect(output).not.toMatch(/srcset|<svg|<video|<form|style="|<link|http-equiv="refresh"/);
	expect(output).toContain("default-src 'none'");
	expect(output).toContain("connect-src 'none'");
	expect(output.indexOf('Content-Security-Policy')).toBeLessThan(output.indexOf('<body>'));
});

it('preserves raster data images and text while stripping data SVG and CID sources', () => {
	const output = sanitizeEmailHtml(
		'<p>Safe text</p><img src="data:image/png;base64,AAAA"><img src="data:image/svg+xml;base64,AAAA"><img src="cid:attachment">'
	);
	expect(output).toContain('data:image/png;base64,AAAA');
	expect(output).not.toContain('svg+xml');
	expect(output).not.toContain('cid:attachment');
	expect(output).toContain('Safe text');
});
