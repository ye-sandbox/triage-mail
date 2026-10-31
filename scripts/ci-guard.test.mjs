import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { scanText } from './ci-guard.mjs';

test('permits reserved mailboxes, package versions and contract field names', () => {
  const text =
    'user@example.com user@sub.example.org user@host.test user@host.invalid\n' +
    'user@localhost.example user@example.net package@1.2.3 client_secret: Option<String>';
  assert.deepEqual(scanText('fixture.txt', text), []);
});

test('rejects non-reserved addresses including misleading reserved suffixes', () => {
  for (const domain of ['mail' + '.com', 'example.com.evil' + '.com', 'notexample' + '.org']) {
    const findings = scanText('fixture.txt', `prefix\nuser@${domain}`);
    assert.deepEqual(findings, [{ path: 'fixture.txt', line: 2, rule: 'non-reserved-email' }]);
    assert.ok(!JSON.stringify(findings).includes(domain));
  }
});

test('rejects high-signal secrets without returning the value', () => {
  const fixtures = [
    ['private-key', '-----BEGIN ' + 'PRIVATE KEY-----'],
    ['github-token', 'gh' + 'p_' + 'A'.repeat(36)],
    ['github-token', 'github_' + 'pat_' + 'A'.repeat(60)],
    ['aws-access-key', 'AK' + 'IA' + 'A'.repeat(16)],
    ['google-api-key', 'AI' + 'za' + 'a'.repeat(35)],
    ['slack-token', 'xo' + 'xb-' + '1'.repeat(20)],
    ['openai-key', 's' + 'k-proj-' + 'a'.repeat(40)],
    ['jwt', 'ey' + 'J' + 'a'.repeat(10) + '.ey' + 'J' + 'b'.repeat(10) + '.' + 'c'.repeat(16)],
  ];
  for (const [rule, value] of fixtures) {
    assert.deepEqual(scanText('fixture.txt', value), [{ path: 'fixture.txt', line: 1, rule }]);
  }
});

test('enforces runes in components and rune modules', () => {
  for (const syntax of [
    'export ' + 'let name;',
    '$' + ': name = value;',
    '<' + 'slot />',
    'on' + ':click={f}',
  ]) {
    for (const path of ['apps/desktop/src/View.svelte', 'apps/desktop/src/state.svelte.ts']) {
      assert.equal(scanText(path, syntax)[0]?.rule, 'legacy-svelte');
    }
    assert.deepEqual(scanText('docs/example.md', syntax), []);
  }
  assert.deepEqual(scanText('apps/desktop/src/View.svelte', 'let { name } = $props();'), []);
});

test('CLI scans tracked files only, skips binary files and exits nonzero on findings', () => {
  const dir = mkdtempSync(join(tmpdir(), 'triage-ci-guard-'));
  const script = resolve('scripts/ci-guard.mjs');
  const value = 'gh' + 'p_' + 'A'.repeat(36);
  try {
    execFileSync('git', ['init', '--quiet', dir]);
    writeFileSync(join(dir, 'tracked.txt'), 'user@example.org');
    writeFileSync(join(dir, 'ignored.txt'), value);
    writeFileSync(join(dir, 'binary.bin'), Buffer.from([0, 1, 2]));
    execFileSync('git', ['add', 'tracked.txt', 'binary.bin'], { cwd: dir });
    assert.equal(spawnSync(process.execPath, [script], { cwd: dir }).status, 0);
    writeFileSync(join(dir, 'tracked.txt'), value);
    const result = spawnSync(process.execPath, [script], { cwd: dir, encoding: 'utf8' });
    assert.equal(result.status, 1);
    assert.match(result.stderr, /tracked.txt:1: github-token/);
    assert.ok(!result.stderr.includes(value));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
