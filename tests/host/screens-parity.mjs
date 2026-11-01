import { createRequire } from "node:module";
import { readFile, readdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { mountApp, textOf } from "./nsh-standin.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "../..");
const fixturesDir = join(repo, "tests/fixtures/screens");
const botDir = process.env.BUWIZ_BOT_DIR;
if (!botDir) throw new Error("BUWIZ_BOT_DIR is required");
const require = createRequire(join(botDir, "server/package.json"));
const esbuild = require("esbuild");
const source = (
  await Promise.all([
    readFile(join(repo, "apps/books/kit.tsx"), "utf8"),
    readFile(join(repo, "apps/cards/shell.tsx"), "utf8"),
    readFile(join(repo, "apps/cards/quiet.tsx"), "utf8"),
  ])
).join("\n");
const record = process.argv.includes("--record");

for (const file of (await readdir(fixturesDir)).filter((name) => name.endsWith(".json")).sort()) {
  const path = join(fixturesDir, file);
  const fixture = JSON.parse(await readFile(path, "utf8"));
  const calls = [];
  let reads = 0;
  let settled = 0;
  const bridge = {
    props: { card: fixture.card },
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
    },
    fit(height) {
      calls.push({ method: "fit", height });
    },
    async open(app, props) {
      calls.push({ method: "open", app, props });
    },
  };
  const screen = mountApp(source, { esbuild, bridge });
  screen.nodes();
  await screen.until(() => settled > 0, "first read");
  await screen.until(() => calls.some((call) => call.method === "ready"), "ready");
  screen.layout(120);
  for (const press of fixture.press || []) {
    try {
      screen.press(press);
    } catch (error) {
      const currentText = screen.nodes().filter((node) => node.host === "Text").map((node) => textOf(node).trim());
      throw new Error(`${file}: ${error.message}; visible text: ${JSON.stringify(currentText)}; calls: ${JSON.stringify(calls)}`);
    }
    if (press === "Try again") await screen.until(() => settled > 1, "retry read");
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  const transcript = {
    text: screen
      .nodes()
      .filter((node) => node.host === "Text")
      .map((node) => textOf(node).trim())
      .filter(Boolean),
    calls,
  };
  if (record) {
    fixture.transcript = transcript;
    await writeFile(path, `${JSON.stringify(fixture, null, 2)}\n`);
  } else if (JSON.stringify(transcript) !== JSON.stringify(fixture.transcript)) {
    throw new Error(`${file}: transcript differs\n${JSON.stringify(transcript, null, 2)}`);
  }
  screen.unmount();
}
console.log(`PASS  quiet screen parity (${(await readdir(fixturesDir)).filter((name) => name.endsWith(".json")).length} fixtures)`);
