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
body {
	margin: 0;
	padding: 16px;
	overflow-wrap: break-word;
}
a {
	color: hsl(210 90% 68%);
}
img {
	max-width: 100%;
	height: auto;
}
`;

// INVARIANT: Never allow script execution, inline event handlers, or remote tracking pixels
// by default when rendering email HTML inside the isolated <iframe sandbox="">.
export function sanitizeEmailHtml(
	rawHtml: string,
	options: SanitizeEmailOptions = {}
): string {
	const allowRemoteImages = options.allowRemoteImages ?? false;

	const sanitizedFragment = DOMPurify.sanitize(rawHtml, {
		FORBID_TAGS: ['script', 'iframe', 'object', 'embed', 'form', 'meta', 'base'],
		FORBID_ATTR: ['onerror', 'onload', 'onclick', 'onmouseover', 'onfocus']
	});

	const pixelSafeBody = allowRemoteImages
		? sanitizedFragment
		: sanitizedFragment.replace(
				/<img\b([^>]*?)\bsrc\s*=\s*(["'])(?:https?:)?\/\/[^"']*\2([^>]*)>/gi,
				'<img$1data-blocked-remote-src="true" alt="[Remote image blocked]"$3>'
			);

	return `<!doctype html><html><head><meta charset="utf-8"><style>${BASE_DARK_MODE_STYLESHEET}</style></head><body>${pixelSafeBody}</body></html>`;
}
