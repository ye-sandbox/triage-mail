import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const email = /(?<![\w@/.-])[a-z\d.!#$%&'*+?^_`{|}~-]+@([a-z\d-]+(?:\.[a-z\d-]+)+)/gi;
const reserved = /(?:^|\.)(?:example\.(?:com|org|net)|example|test|invalid|localhost)$/i;
const patterns = [
  ['private-key', /-----BEGIN (?:RSA |EC |OPENSSH |DSA |ENCRYPTED )?PRIVATE KEY-----/],
  ['github-token', /\b(?:gh[pousr]_[A-Za-z\d]{36,}|github_pat_[A-Za-z\d_]{60,})\b/],
  ['aws-access-key', /\b(?:AKIA|ASIA)[A-Z\d]{16}\b/],
  ['google-api-key', /\bAIza[A-Za-z\d_-]{35}\b/],
  ['slack-token', /\bxox[baprs]-[A-Za-z\d-]{20,}\b/],
  ['openai-key', /\bsk-(?:proj-|svcacct-)?[A-Za-z\d_-]{40,}\b/],
  ['jwt', /\beyJ[A-Za-z\d_-]{10,}\.eyJ[A-Za-z\d_-]{10,}\.[A-Za-z\d_-]{16,}\b/],
];
const legacy = /\bexport\s+let\b|^\s*\$\s*:|<slot(?:\s|\/?>)|\bon:[a-z]+\s*=/;

export function scanText(path, content) {
  const findings = [];
  // INVARIANT: Report locations and rule names only; never echo matched secret/PII values.
  content.split('\n').forEach((line, index) => {
    for (const match of line.matchAll(email)) {
      // Scoped npm versions (e.g. package@1.2.3 or package@1.0.0-next.7) are not mailbox addresses.
      if (
        !/^(?:[\d.]+|\d+\.\d+\.\d+(?:-[a-z\d-]+(?:\.\d+)?)?)$/i.test(match[1]) &&
        !reserved.test(match[1])
      ) {
        findings.push({ path, line: index + 1, rule: 'non-reserved-email' });
      }
    }
    for (const [rule, pattern] of patterns) {
      if (pattern.test(line)) findings.push({ path, line: index + 1, rule });
    }
    if (
      path.startsWith('apps/desktop/src/') &&
      /\.svelte(?:\.ts)?$/.test(path) &&
      legacy.test(line)
    ) {
      findings.push({ path, line: index + 1, rule: 'legacy-svelte' });
    }
  });
  return findings;
}

export function scanRepository() {
  const root = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim();
  const files = execFileSync('git', ['ls-files', '-z'], { cwd: root, encoding: 'utf8' })
    .split('\0')
    .filter(Boolean);
  const findings = [];
  let textFiles = 0;
  for (const path of files) {
    const data = readFileSync(resolve(root, path));
    if (data.includes(0)) continue;
    textFiles++;
    findings.push(...scanText(path, data.toString('utf8')));
  }
  for (const item of findings) console.error(`${item.path}:${item.line}: ${item.rule}`);
  console.log(`Policy scan: ${textFiles} tracked text files, ${findings.length} findings.`);
  return findings.length === 0 ? 0 : 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exitCode = scanRepository();
}
