// Samples only the child and its descendants; never reads mailbox or vault data.
import { spawn } from 'node:child_process';
import { readdir, readFile } from 'node:fs/promises';
import { createInterface } from 'node:readline';

const [binary, duration] = process.argv.slice(2);
const durationMs = duration === undefined ? 90_000 : Number(duration);
if (!Number.isFinite(durationMs) || durationMs < 1000 || durationMs > 90_000)
	throw new Error('Duration must be between 1000 and 90000 ms');
let timedStop = false;
if (!binary)
	throw new Error(
		'Usage: node tools/performance/sample-native.mjs BINARY [DURATION_MS]'
	);
// INVARIANT: Give this disposable fixture its own process group so a wrapper timeout
// also terminates native/WebKit descendants that retain inherited stdout pipes.
const child = spawn(binary, [], {
	stdio: ['ignore', 'pipe', 'inherit'],
	detached: process.platform !== 'win32'
});
let phase = 'startup';
const reports = [];
const memory = [];
const lines = createInterface({ input: child.stdout });
lines.on('line', (line) => {
	try {
		const report = JSON.parse(line);
		reports.push(report);
		if (typeof report.phase === 'string') {
			phase = report.phase;
			process.stderr.write(`Native phase: ${phase}\n`);
		}
	} catch {
		process.stderr.write(`${line}\n`);
	}
});
async function sample() {
	const processes = await Promise.all(
		(await readdir('/proc'))
			.filter((s) => /^\d+$/.test(s))
			.map(async (pid) => {
				try {
					const status = await readFile(`/proc/${pid}/status`, 'utf8');
					return {
						pid: Number(pid),
						parent: Number(status.match(/^PPid:\s+(\d+)/m)?.[1]),
						rss: Number(status.match(/^VmRSS:\s+(\d+)/m)?.[1] ?? 0)
					};
				} catch {
					return null;
				}
			})
	);
	const descendants = new Set([child.pid]);
	let before;
	do {
		before = descendants.size;
		for (const process of processes)
			if (process && descendants.has(process.parent))
				descendants.add(process.pid);
	} while (before !== descendants.size);
	let pss = 0;
	let readable = 0;
	let rss = 0;
	for (const process of processes) {
		if (!process || !descendants.has(process.pid)) continue;
		rss += process.rss;
		try {
			const rollup = await readFile(
				`/proc/${process.pid}/smaps_rollup`,
				'utf8'
			);
			pss += Number(rollup.match(/^Pss:\s+(\d+)/m)?.[1] ?? 0);
			readable++;
		} catch {
			/* Process may exit between discovery and sampling. */
		}
	}
	memory.push({
		phase,
		time_ms: Date.now(),
		rss_kib: rss,
		pss_kib: pss,
		processes: descendants.size,
		pss_readable: readable
	});
}
let sampling = false;
const timer = setInterval(() => {
	if (sampling) return;
	sampling = true;
	void sample().finally(() => {
		sampling = false;
	});
}, 250);
const timeout = setTimeout(() => {
	timedStop = true;
	try {
		if (process.platform === 'win32') child.kill('SIGTERM');
		else if (child.pid) process.kill(-child.pid, 'SIGTERM');
	} catch (error) {
		if (error.code !== 'ESRCH') throw error;
	}
}, durationMs);
child.on('error', (error) => {
	process.stderr.write(`${error}\n`);
});
child.on('close', (code, signal) => {
	clearInterval(timer);
	clearTimeout(timeout);
	console.log(JSON.stringify({ code, signal, reports, memory }, null, 2));
	if (
		duration === undefined
			? code !== 0 || !reports.some((report) => report.phase === 'complete')
			: !timedStop || signal !== 'SIGTERM' || memory.length === 0
	)
		process.exitCode = 1;
});
