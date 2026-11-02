// Fixture-only theme data comes from production CSS, without duplicated color literals.
import { readFile, writeFile } from 'node:fs/promises';
const css = await readFile('apps/desktop/src/app.css', 'utf8');
const roots = [...css.matchAll(/:root\s*\{([^}]+)\}/g)].slice(0, 2);
if (roots.length !== 2)
	throw new Error('Expected production light and dark token blocks');
const themes = Object.fromEntries(
	roots.map((match, index) => [
		index ? 'dark' : 'light',
		Object.fromEntries(
			[...match[1].matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)].map(
				([, name, value]) => [name, value.trim()]
			)
		)
	])
);
await writeFile(
	'apps/desktop/src/routes/__performance/visual-themes.fixtures.ts',
	`export const themes = ${JSON.stringify(themes)} as const;\n`
);
