import DOMPurify from 'dompurify';

export interface SanitizeEmailOptions {
	allowRemoteImages?: boolean;
	theme?: 'light' | 'dark';
}

function buildStylesheet(theme?: 'light' | 'dark'): string {
	const lightPalette = `
	--reader-bg: #ffffff;
	--reader-fg: #111827;
	--reader-muted: #525d6d;
	--reader-border: #e2e6ec;
	--reader-link: #2563eb;
	--reader-code-bg: #f1f3f6;
	--reader-scrollbar-thumb: rgba(82, 93, 109, 0.24);
	--reader-scrollbar-thumb-hover: rgba(82, 93, 109, 0.42);
	--reader-scrollbar-thumb-active: rgba(82, 93, 109, 0.58);`;

	const darkPalette = `
	--reader-bg: #121721;
	--reader-fg: #eef2f7;
	--reader-muted: #98a4b8;
	--reader-border: #242d3d;
	--reader-link: #60a5fa;
	--reader-code-bg: #1c2433;
	--reader-scrollbar-thumb: rgba(152, 164, 184, 0.24);
	--reader-scrollbar-thumb-hover: rgba(152, 164, 184, 0.42);
	--reader-scrollbar-thumb-active: rgba(152, 164, 184, 0.58);`;

	const rootVars =
		theme === 'light'
			? `color-scheme: light;${lightPalette}`
			: theme === 'dark'
				? `color-scheme: dark;${darkPalette}`
				: `color-scheme: dark light;${darkPalette}`;

	const mediaFallback =
		theme === undefined ? `@media (prefers-color-scheme: light) { :root {${lightPalette}\n} }` : '';

	return `
:root {${rootVars}
	font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
	line-height: 1.6;
}
${mediaFallback}
*::-webkit-scrollbar { width: 8px; height: 8px; }
*::-webkit-scrollbar-track { background: transparent; }
*::-webkit-scrollbar-thumb {
	background-color: var(--reader-scrollbar-thumb);
	border-radius: 9999px;
	border: 2px solid transparent;
	background-clip: content-box;
}
*::-webkit-scrollbar-thumb:hover { background-color: var(--reader-scrollbar-thumb-hover); }
*::-webkit-scrollbar-thumb:active { background-color: var(--reader-scrollbar-thumb-active); }
*::-webkit-scrollbar-corner { background: transparent; }
*::-webkit-scrollbar-button { display: none; width: 0; height: 0; }
@supports not selector(::-webkit-scrollbar) {
	* {
		scrollbar-width: thin;
		scrollbar-color: var(--reader-scrollbar-thumb) transparent;
	}
}
body {
	margin: 0;
	padding: 16px;
	background: var(--reader-bg);
	color: var(--reader-fg);
	font-size: 14px;
	overflow-wrap: anywhere;
	word-break: break-word;
}
a { color: var(--reader-link); text-decoration: underline; }
blockquote {
	margin: 12px 0;
	padding: 4px 12px;
	border-left: 2px solid var(--reader-border);
	color: var(--reader-muted);
}
pre, code {
	font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
	background: var(--reader-code-bg);
	border-radius: 4px;
}
code { padding: 2px 5px; font-size: 12px; }
pre { padding: 12px; overflow-x: auto; white-space: pre-wrap; }
table { border-collapse: collapse; width: 100%; max-width: 100%; margin: 12px 0; }
th, td { border: 1px solid var(--reader-border); padding: 8px; text-align: left; }
hr { border: 0; border-top: 1px solid var(--reader-border); margin: 16px 0; }
img { max-width: 100%; height: auto; border-radius: 6px; display: block; margin: 8px 0; }
`;
}

export function hasBlockedRemoteImages(htmlRaw: string): boolean {
	if (!htmlRaw) return false;
	const imgMatches = htmlRaw.matchAll(/<img\b[^>]*>/gi);
	for (const match of imgMatches) {
		const tag = match[0];
		const srcMatch = /\bsrc\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))/i.exec(tag);
		if (!srcMatch) continue;
		const src = (srcMatch[1] ?? srcMatch[2] ?? srcMatch[3] ?? '').trim();
		const isRasterData = /^data:image\/(?:png|jpeg|gif|webp);base64,[a-z0-9+/=\s]+$/i.test(src);
		if (src && !isRasterData) {
			return true;
		}
	}
	return false;
}

export function sanitizeEmailHtml(rawHtml: string, options: SanitizeEmailOptions = {}): string {
	// INVARIANT: Sender CSS and URL-bearing attributes are excluded, not rewritten with regex.
	// Only raster data images (or explicitly opted-in HTTPS images) survive; CID images stay lazy.
	const fragment = DOMPurify.sanitize(rawHtml, {
		ALLOWED_TAGS: [
			'p',
			'div',
			'span',
			'br',
			'hr',
			'h1',
			'h2',
			'h3',
			'h4',
			'h5',
			'h6',
			'b',
			'strong',
			'i',
			'em',
			'u',
			's',
			'blockquote',
			'pre',
			'code',
			'ul',
			'ol',
			'li',
			'table',
			'thead',
			'tbody',
			'tfoot',
			'tr',
			'td',
			'th',
			'caption',
			'a',
			'img',
			'dl',
			'dt',
			'dd'
		],
		ALLOWED_ATTR: ['alt', 'title', 'colspan', 'rowspan', 'dir', 'lang', 'src'],
		ALLOW_DATA_ATTR: false,
		ALLOW_ARIA_ATTR: false,
		RETURN_DOM_FRAGMENT: true
	});
	for (const element of fragment.querySelectorAll('[src]')) {
		const src = element.getAttribute('src') ?? '';
		const raster = /^data:image\/(?:png|jpeg|gif|webp);base64,[a-z0-9+/=\s]+$/i.test(src);
		const remote = options.allowRemoteImages && /^https:\/\//i.test(src);
		if (element.tagName !== 'IMG' || (!raster && !remote)) {
			element.removeAttribute('src');
			element.setAttribute('data-blocked-remote-src', 'true');
			if (!element.getAttribute('alt')) element.setAttribute('alt', '[Image blocked]');
		}
	}
	const container = document.createElement('div');
	container.append(fragment);
	// INVARIANT: CSP precedes sender content; the caller also uses an empty iframe sandbox.
	const imgSources = options.allowRemoteImages ? 'data: https:' : 'data:';
	const csp = `default-src 'none'; script-src 'none'; style-src 'unsafe-inline'; img-src ${imgSources}; connect-src 'none'; font-src 'none'; media-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'`;
	const stylesheet = buildStylesheet(options.theme);
	return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="${csp}"><meta name="referrer" content="no-referrer"><style>${stylesheet}</style></head><body>${container.innerHTML}</body></html>`;
}
