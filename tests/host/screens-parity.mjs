import { createRequire } from "node:module";
import { access, readFile, readdir, writeFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";

import { mountApp, prepareApp, textOf } from "./nsh-standin.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "../..");
const screens = [
  {
    id: "quiet",
    directory: "quiet",
    source: ["quiet"],
    sourceFiles: ["apps/cards/quiet.tsx"],
    fixtures: [
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
    ],
  },
  {
    id: "ledger",
    directory: "ledger",
    source: ["books"],
    sourceFiles: ["apps/books/ledger.tsx"],
    fixtures: ["ledger-cases.json"],
  },
];
const botDir = process.env.BUWIZ_BOT_DIR;
if (!botDir) throw new Error("BUWIZ_BOT_DIR is required");
const require = createRequire(join(botDir, "server/package.json"));
const esbuild = require("esbuild");
const record = process.argv.includes("--record");

const percentile = (values, p) => values.slice().sort((a, b) => a - b)[Math.min(values.length - 1, Math.ceil(values.length * p) - 1)];
const ledgerShellTexts = new Set([
  "B",
  "Books",
  "Today",
  "Unpaid",
  "Reconcile",
  "Reports",
  "Forms",
  "People",
  "Accounts",
  "Tags",
  "Payroll",
  "Certificates",
  "Settings",
  "Money",
  "Results",
  "Records",
  "BIR",
  "Sample Books",
  "Cash basis · 2026",
  "Opening Books…",
]);

for (const config of screens) {
  const fixturesDir = join(repo, "tests/fixtures/screens", config.directory);
  const files = (await readdir(fixturesDir)).filter((name) => name.endsWith(".json")).sort();
  if (JSON.stringify(files) !== JSON.stringify([...config.fixtures].sort())) {
    throw new Error(`${config.id} fixture set differs: ${JSON.stringify(files)}`);
  }

  const hasSource = await Promise.all(
    config.sourceFiles.map(async (path) => {
      try {
        await access(join(repo, path));
        return true;
      } catch {
        return false;
      }
    }),
  ).then((results) => results.every(Boolean));
  const source = hasSource
    ? execFileSync(join(repo, "target/release/books-mini-app"), ["app-source", ...config.source], {
        encoding: "utf8",
      })
    : null;
  if (!source) {
    console.log(`SKIP TSX ${config.id}: source was deleted; checking frozen transcript goldens`);
  }

  const timings = [];
  for (const file of files) {
    const path = join(fixturesDir, file);
    const fixtureSet = JSON.parse(await readFile(path, "utf8"));
    const fixtureCases = fixtureSet.cases || [fixtureSet];
    if (!source) continue;
    for (const fixtureCase of fixtureCases) {
    const defaultAnswers = fixtureSet.defaults?.answers;
    const caseAnswers = fixtureCase.answers;
    const fixture = {
      ...fixtureSet.defaults,
      ...fixtureCase,
      answers:
        Array.isArray(caseAnswers) || Array.isArray(defaultAnswers)
          ? caseAnswers || defaultAnswers
          : { ...(defaultAnswers || {}), ...(caseAnswers || {}) },
    };
    const caseName = fixture.name || file;
    const run = async (prepared = false, transformedSource = null) => {
      const calls = [];
      let requestCounter = 0;
      const answerIndexes = new Map();
      const settledByTool = new Map();
      let inflight = 0;
      let lastActivity = Date.now();
      let readyAt;
      let rowAt;
      const startedAt = performance.now();
      const answerFor = (tool) => {
        const answers = Array.isArray(fixture.answers)
          ? fixture.answers
          : fixture.answers[tool] || [];
        const index = answerIndexes.get(tool) || 0;
        answerIndexes.set(tool, index + 1);
        const answer = answers[Math.min(index, answers.length - 1)];
        if (!answer) throw new Error(`${file}: no synthetic answer for ${tool} call ${index + 1}`);
        return answer;
      };
      const bridge = {
        props: fixture.props,
        async call(tool, args) {
          const tracked = config.id !== "ledger" || !["books_status", "books_book_get"].includes(tool);
          if (tracked) {
            const normalizedArgs = structuredClone(args);
            if (typeof normalizedArgs.client_request_id === "string") {
              const match = /^bk-(\d{8})-(\d+)$/.exec(normalizedArgs.client_request_id);
              if (!match) throw new Error(`invalid client_request_id for ${tool}`);
              requestCounter += 1;
              if (Number(match[2]) !== requestCounter)
                throw new Error(`client_request_id counter differs for ${tool}`);
              normalizedArgs.client_request_id = `bk-<open>-${requestCounter}`;
            }
            calls.push({ method: "call", tool, args: normalizedArgs });
          }
          inflight += 1;
          lastActivity = Date.now();
          try {
            const answer = answerFor(tool);
            if (answer.error) throw new Error(answer.error);
            return answer.data;
          } finally {
            inflight -= 1;
            settledByTool.set(tool, (settledByTool.get(tool) || 0) + 1);
            lastActivity = Date.now();
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
          calls.push({
            method: "open",
            app: "books",
            props: request.props,
          });
          if (fixture.open_error) throw new Error(fixture.open_error);
        },
      };
      const screen = mountApp(prepared ? transformedSource : source, {
        esbuild,
        bridge,
        prepared,
      });
      const shownText = new Set();
      const captureText = () => {
        for (const node of screen.nodes()) {
          if (node.host !== "Text") continue;
          const value = textOf(node).trim();
          if (value) shownText.add(value);
        }
      };
      screen.nodes();
      captureText();
      let retryBeforeRow = false;
      if (config.id === "quiet") {
        await screen.until(() => (settledByTool.get("books_nudge") || 0) > 0, "first read");
        await screen.until(() => readyAt !== undefined, "ready");
        screen.layout(120);
      } else {
        await screen.until(() => {
          captureText();
          return (settledByTool.get("books_ledger_list") || 0) > 0;
        }, "first ledger list");
        await screen.until(
          () => {
            captureText();
            return inflight === 0 && Date.now() - lastActivity >= 40;
          },
          "ledger list to settle",
          2000,
        );
        captureText();
        if (fixture.answers.books_ledger_list[0]?.error) {
          retryBeforeRow = true;
          const previousCount = settledByTool.get("books_ledger_list") || 0;
          screen.press("Try again");
          await screen.until(
            () => (settledByTool.get("books_ledger_list") || 0) > previousCount,
            "ledger retry",
          );
          await screen.until(
            () => {
              captureText();
              return inflight === 0 && Date.now() - lastActivity >= 40;
            },
            "ledger retry to settle",
            2000,
          );
        }
        const timingText =
          fixture.timing_text ||
          fixture.answers.books_ledger_list.find((answer) => answer.data)?.data?.rows?.[0]?.title;
        if (timingText) {
          try {
            await screen.until(() => {
              captureText();
              return screen.hasText(timingText);
            }, "first row text");
          } catch (error) {
            const currentText = screen.nodes().filter((node) => node.host === "Text").map((node) => textOf(node).trim());
            throw new Error(`${file}: ${error.message}; visible text: ${JSON.stringify(currentText)}; settled: ${JSON.stringify([...settledByTool])}; calls: ${JSON.stringify(calls)}`);
          }
          rowAt = performance.now() - startedAt;
        }
      }
      const steps = fixture.steps || (fixture.press || []).map((press) => ({ press }));
      for (const [index, step] of steps.entries()) {
        if (retryBeforeRow && index === 0) continue;
        try {
          if (typeof step.press === "string") screen.press(step.press, step.occurrence ?? "last");
          if (typeof step.contains === "string")
            screen.pressContaining(step.contains, step.occurrence ?? "last");
          if (typeof step.type === "string") screen.type(step.type, step.value);
          if (step.expect) await screen.until(() => screen.hasText(step.expect), `text ${step.expect}`);
          await screen.until(
            () => {
              captureText();
              return inflight === 0 && Date.now() - lastActivity >= 40;
            },
            "screen calls to settle",
            2000,
          );
        } catch (error) {
          const currentText = screen.nodes().filter((node) => node.host === "Text").map((node) => textOf(node).trim());
          throw new Error(`${file}: ${error.message}; visible text: ${JSON.stringify(currentText)}; calls: ${JSON.stringify(calls)}`);
        }
      }
      captureText();
      const texts = [...shownText].filter(
        (value) =>
          config.id !== "ledger" ||
          (!ledgerShellTexts.has(value) && !/^\d+$/.test(value)),
      );
      const transcript = { text: texts, calls };
      screen.unmount();
      return { transcript, readyAt: readyAt ?? rowAt };
    };
      const result = await run();
    if (record) {
      fixtureCase.transcript = result.transcript;
    } else if (JSON.stringify(result.transcript) !== JSON.stringify(fixture.transcript)) {
      throw new Error(`${caseName}: transcript differs\n${JSON.stringify(result.transcript, null, 2)}`);
    }
    if (!record) {
      const transformedSource = prepareApp(source, { esbuild });
      for (let i = 0; i < 50; i += 1) {
        timings.push([caseName, "compile+mount", (await run()).readyAt]);
      }
      for (let i = 0; i < 50; i += 1) {
        timings.push([caseName, "mount-only", (await run(true, transformedSource)).readyAt]);
      }
    }
    }
    if (record && fixtureSet.cases) {
      await writeFile(path, `${JSON.stringify(fixtureSet, null, 2)}\n`);
    } else if (record) {
      fixtureSet.transcript = fixtureCases[0].transcript;
      await writeFile(path, `${JSON.stringify(fixtureSet, null, 2)}\n`);
    }
  }
  if (source) {
    let fixtureCount = 0;
    for (const file of files) {
      const fixtureSet = JSON.parse(await readFile(join(fixturesDir, file), "utf8"));
      fixtureCount += fixtureSet.cases?.length || 1;
    }
    if (!record) {
      for (const caseName of new Set(timings.map(([name]) => name))) {
        for (const mode of ["compile+mount", "mount-only"]) {
          const values = timings
            .filter(([name, actualMode]) => name === caseName && actualMode === mode)
            .map(([, , value]) => value);
          if (!values.length) continue;
          console.log(`TIMING TSX ${config.id} ${caseName} ${mode}: median ${percentile(values, 0.5).toFixed(2)} ms, p95 ${percentile(values, 0.95).toFixed(2)} ms (50 runs)`);
        }
      }
    }
    console.log(`PASS  ${config.id} screen TSX parity (${fixtureCount} fixtures)`);
  } else {
    console.log(`PASS  ${config.id} frozen transcript set (${files.length} fixtures)`);
  }
}
