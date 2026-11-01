import { createRequire } from "node:module";
import { access, readFile, readdir, writeFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";

import { mountApp, textOf } from "./nsh-standin.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "../..");
const fixturesDir = join(repo, "tests/fixtures/screens/quiet");
const expectedFixtures = [
  "handoff-0.json",
  "handoff-1.json",
  "handoff-2.json",
  "month-closed.json",
  "month-open-1.json",
  "month-open-2.json",
  "open-failure.json",
  "read-error.json",
  "retry-read-error.json",
  "tidy-0.json",
  "tidy-1.json",
  "tidy-3.json",
];
const botDir = process.env.BUWIZ_BOT_DIR;
if (!botDir) throw new Error("BUWIZ_BOT_DIR is required");
const require = createRequire(join(botDir, "server/package.json"));
const esbuild = require("esbuild");
const record = process.argv.includes("--record");

const files = (await readdir(fixturesDir)).filter((name) => name.endsWith(".json")).sort();
if (JSON.stringify(files) !== JSON.stringify([...expectedFixtures].sort())) {
  throw new Error(`quiet fixture set differs: ${JSON.stringify(files)}`);
}

let hasQuietTsx = true;
try {
  await access(join(repo, "apps/cards/quiet.tsx"));
} catch {
  hasQuietTsx = false;
}
let source;
if (hasQuietTsx) {
  source = execFileSync(join(repo, "target/release/books-mini-app"), ["app-source", "quiet"], { encoding: "utf8" });
} else {
  console.log("SKIP TSX quiet: quiet.tsx was deleted; checking frozen transcript goldens");
}

const percentile = (values, p) => values.slice().sort((a, b) => a - b)[Math.min(values.length - 1, Math.ceil(values.length * p) - 1)];
const timings = [];
for (const file of files) {
  const path = join(fixturesDir, file);
  const fixture = JSON.parse(await readFile(path, "utf8"));
  if (!source) continue;
  const run = async () => {
    const calls = [];
    let reads = 0;
    let settled = 0;
    let readyAt;
    const startedAt = performance.now();
    const bridge = {
      props: fixture.props,
      async call(tool, args) {
        calls.push({ method: "call", tool, args });
        const answer = fixture.answers[Math.min(reads++, fixture.answers.length - 1)];
        try {
          if (answer.error) throw new Error(answer.error);
          return answer.data;
        } finally {
          settled += 1;
        }
      },
      ready() {
        calls.push({ method: "ready" });
        readyAt ??= performance.now() - startedAt;
      },
      fit() {
        calls.push({ method: "fit" });
      },
      async open(request) {
        calls.push({ method: "open", app: request.app, props: request.props });
        if (fixture.open_error) throw new Error(fixture.open_error);
      },
    };
    const screen = mountApp(source, { esbuild, bridge });
    screen.nodes();
    await screen.until(() => settled > 0, "first read");
    await screen.until(() => readyAt !== undefined, "ready");
    screen.layout(120);
    for (const press of fixture.press || []) {
      try {
        screen.press(press);
      } catch (error) {
        const currentText = screen.nodes().filter((node) => node.host === "Text").map((node) => textOf(node).trim());
        throw new Error(`${file}: ${error.message}; visible text: ${JSON.stringify(currentText)}; calls: ${JSON.stringify(calls)}`);
      }
      if (press === "Try again") await screen.until(() => settled > 1, "retry read");
      if (fixture.open_error) await screen.until(() => screen.hasText(fixture.open_error), "open error toast");
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
    const transcript = {
      text: screen.nodes().filter((node) => node.host === "Text").map((node) => textOf(node).trim()).filter(Boolean),
      calls,
    };
    screen.unmount();
    return { transcript, readyAt };
  };
  const result = await run();
  if (record) {
    fixture.transcript = result.transcript;
    await writeFile(path, `${JSON.stringify(fixture, null, 2)}\n`);
  } else if (source && JSON.stringify(result.transcript) !== JSON.stringify(fixture.transcript)) {
    throw new Error(`${file}: transcript differs\n${JSON.stringify(result.transcript, null, 2)}`);
  }
  for (let i = 0; i < 50; i += 1) timings.push([file, (await run()).readyAt]);
}
if (source) {
  for (const file of files) {
    const values = timings.filter(([name]) => name === file).map(([, value]) => value);
    console.log(`TIMING TSX quiet ${file}: median ${percentile(values, 0.5).toFixed(2)} ms, p95 ${percentile(values, 0.95).toFixed(2)} ms (50 runs)`);
  }
  console.log(`PASS  quiet screen TSX parity (${files.length} fixtures)`);
} else {
  console.log(`PASS  quiet frozen transcript set (${files.length} fixtures)`);
}
