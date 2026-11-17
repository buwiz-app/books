// A scripted stand-in for the model, on loopback, for the host tests that need the assistant's side: the broker's tool
// loop talks to it, and what the broker offers the model and answers to its tool calls is read from the requests it
// sends.
import { createServer } from "node:http";

// a scripted stand-in for the model on loopback: Messages over SSE. A script
// is the tool calls it makes when asked; a request that carries tool results gets the closing answer.
// The newest user text with [[script:<name>]] picks the script.
// whether the request's last message carries tool results
export const hasToolResults = (body) => (body.messages?.at(-1)?.content ?? []).some?.((b) => b.type === "tool_result") === true;

export async function scriptedModel(port) {
  const scripts = new Map();
  const requests = [];
  const server = createServer((req, res) => {
    const chunks = [];
    req.on("data", (c) => chunks.push(c));
    req.on("end", () => {
      let body = {};
      try { body = JSON.parse(Buffer.concat(chunks).toString("utf8")); } catch {}
      if (!/\/v1\/messages$/.test(req.url)) { res.writeHead(404).end("{}"); return; }
      const texts = (body.messages ?? []).filter((m) => m.role === "user").map((m) => typeof m.content === "string" ? m.content : (m.content ?? []).filter((b) => b.type === "text").map((b) => b.text).join("\n"));
      const name = texts.reverse().map((t) => /\[\[script:([\w.-]+)\]\]/.exec(t)?.[1]).find(Boolean);
      const calls = scripts.get(name);
      requests.push({ script: name, body });
      if (!calls) { res.writeHead(200, { "content-type": "application/json" }); res.end(JSON.stringify({ id: "msg_x", type: "message", role: "assistant", model: body.model, content: [{ type: "text", text: "ok" }], stop_reason: "end_turn", stop_sequence: null, usage: { input_tokens: 1, output_tokens: 1 } })); return; }
      const content = hasToolResults(body) ? [{ type: "text", text: "Done." }] : calls.map((b, i) => ({ ...b, id: `toolu_${i}` }));
      const stop_reason = content.some((b) => b.type === "tool_use") ? "tool_use" : "end_turn";
      const message = { id: "msg_x", type: "message", role: "assistant", model: body.model, content, stop_reason, stop_sequence: null, usage: { input_tokens: 1, output_tokens: 1 } };
      if (!body.stream) { res.writeHead(200, { "content-type": "application/json" }); res.end(JSON.stringify(message)); return; }
      res.writeHead(200, { "content-type": "text/event-stream" });
      const ev = (type, data) => res.write(`event: ${type}\ndata: ${JSON.stringify({ type, ...data })}\n\n`);
      ev("message_start", { message: { ...message, content: [], stop_reason: null } });
      content.forEach((b, index) => {
        if (b.type === "text") {
          ev("content_block_start", { index, content_block: { type: "text", text: "" } });
          ev("content_block_delta", { index, delta: { type: "text_delta", text: b.text } });
        } else {
          ev("content_block_start", { index, content_block: { type: "tool_use", id: b.id, name: b.name, input: {} } });
          ev("content_block_delta", { index, delta: { type: "input_json_delta", partial_json: JSON.stringify(b.input ?? {}) } });
        }
        ev("content_block_stop", { index });
      });
      ev("message_delta", { delta: { stop_reason, stop_sequence: null }, usage: { output_tokens: 1 } });
      ev("message_stop", {});
      res.end();
    });
  });
  await new Promise((resolve, reject) => { server.once("error", reject); server.listen(Number(port), "127.0.0.1", resolve); });
  return {
    requests,
    script: (name, calls) => scripts.set(name, calls),
    close: () => new Promise((r) => { server.closeAllConnections?.(); server.close(r); }),
  };
}
