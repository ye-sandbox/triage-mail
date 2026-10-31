import DOMPurify from 'dompurify';

export interface SanitizeEmailOptions {
	allowRemoteImages?: boolean;
}

const BASE_DARK_MODE_STYLESHEET = `
:root {
	color-scheme: dark light;
	font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
	line-height: 1.5;
	color: hsl(210 20% 96%);
	background-color: hsl(224 24% 10%);
}
body { margin: 0; padding: 16px; overflow-wrap: anywhere; }
a { color: hsl(210 90% 68%); }
img, table { max-width: 100%; }
img { height: auto; }
pre { white-space: pre-wrap; }
`;

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
	return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="${csp}"><meta name="referrer" content="no-referrer"><style>${BASE_DARK_MODE_STYLESHEET}</style></head><body>${container.innerHTML}</body></html>`;
}
