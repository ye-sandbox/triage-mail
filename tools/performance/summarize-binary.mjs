// Offline CSV reduction; reject incomplete sweeps instead of reporting partial percentiles.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const lines = readFileSync(
  process.argv[2] ?? "docs/performance/binary-quantization.csv",
  "utf8",
)
  .trim()
  .split("\n")
  .filter((line) => !line.startsWith("#"));
const header = lines.shift().split(",");
const groups = new Map();
for (const line of lines) {
  const values = line.split(",");
  assert.equal(values.length, header.length);
  const row = Object.fromEntries(header.map((name, i) => [name, values[i]]));
  const key = [row.view, row.seed, row.offset, row.algorithm, row.budget].join(
    "/",
  );
  if (!groups.has(key)) groups.set(key, []);
  groups.get(key).push(row);
}
assert.equal(groups.size, 64, "expected 16 workloads × four algorithms");
assert.equal(
  lines.length,
  1984,
  "expected one reopened and 30 warm samples per group",
);
for (const rows of groups.values()) {
  assert.equal(rows.filter((r) => r.cache === "reopened").length, 1);
  assert.equal(rows.filter((r) => r.cache === "warm").length, 30);
}
function summarize(rows) {
  const first = rows[0];
  const warm = rows.filter((r) => r.cache === "warm");
  const times = warm.map((r) => Number(r.ms)).sort((a, b) => a - b);
  const hits = warm.map((r) => Number(r.oracle_hits));
  const low = Math.min(...hits),
    high = Math.max(...hits);
  const recall =
    first.oracle_count === "0"
      ? "n/a"
      : `${low === high ? low : `${low}–${high}`}/${first.oracle_count}`;
  const equivalent = warm.filter((r) => r.equivalent === "1").length;
  return `| ${first.view} | ${first.seed} | ${first.eligible} | ${first.algorithm} ${first.budget} | ${((times[14] + times[15]) / 2).toFixed(3)} | ${times[28].toFixed(3)} | ${recall} | ${equivalent}/30 |`;
}
for (const offset of ["0", "4000"]) {
  console.log(`### Offset ${offset}\n`);
  console.log(
    "| View | Seed | Eligible | Algorithm / budget | Warm p50 ms | Warm p95 ms | Oracle hits/page | Ordered equality |",
  );
  console.log("|---|---:|---:|---|---:|---:|---|---|");
  for (const rows of groups.values()) {
    if (rows[0].offset === offset) console.log(summarize(rows));
  }
  if (offset === "0") console.log("");
}
