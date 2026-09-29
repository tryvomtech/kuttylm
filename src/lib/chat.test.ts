// Run: npm run test:ui
// The last test needs a running llama-server: set KUTTYLM_TEST_CHAT_URL and KUTTYLM_TEST_CHAT_KEY.
import { test } from "node:test";
import assert from "node:assert/strict";
import { applyChunk, ChatError, errorMessage, streamChat, type ChatReply } from "./chat.ts";

const empty = (): ChatReply => ({ content: "", reasoning: "", tokensPerSecond: null, finishReason: null });

test("chunks build the answer", () => {
  const r = empty();
  applyChunk(r, { choices: [{ delta: { content: "Hel" } }] });
  applyChunk(r, { choices: [{ delta: { content: "lo", reasoning_content: "hmm" } }] });
  applyChunk(r, { choices: [{ delta: {}, finish_reason: "stop" }], timings: { predicted_per_second: 25.2 } });
  assert.equal(r.content, "Hello");
  assert.equal(r.reasoning, "hmm");
  assert.equal(r.finishReason, "stop");
  assert.equal(r.tokensPerSecond, 25.2);
});

test("error chunk becomes a friendly error", () => {
  assert.throws(() => applyChunk(empty(), { error: { message: "boom" } }), ChatError);
  assert.match(errorMessage({ type: "exceed_context_size_error" }), /Start a new chat/);
});

const url = process.env.KUTTYLM_TEST_CHAT_URL;
test("real stream from llama-server", { skip: !url }, async () => {
  let updates = 0;
  const reply = await streamChat({
    baseUrl: url!,
    apiKey: process.env.KUTTYLM_TEST_CHAT_KEY ?? null,
    messages: [{ role: "user", content: "Name three colours." }],
    signal: new AbortController().signal,
    onUpdate: () => updates++,
  });
  console.log(`answer: ${JSON.stringify(reply.content.slice(0, 120))}, ${updates} updates, ${reply.tokensPerSecond?.toFixed(1)} tok/s`);
  assert.ok(reply.content.length > 0);
  assert.ok(updates > 1, "answer should arrive in pieces");
  assert.equal(reply.finishReason, "stop");
});

test("Stop ends the answer early", { skip: !url }, async () => {
  const controller = new AbortController();
  const reply = await streamChat({
    baseUrl: url!,
    apiKey: process.env.KUTTYLM_TEST_CHAT_KEY ?? null,
    messages: [{ role: "user", content: "Write a long story about a cat." }],
    signal: controller.signal,
    onUpdate: (r) => {
      if (r.content.length > 20) controller.abort();
    },
  });
  assert.equal(reply.finishReason, "stopped");
  assert.ok(reply.content.length < 400);
});

test("wrong key gives a clear message", { skip: !url }, async () => {
  await assert.rejects(
    streamChat({
      baseUrl: url!,
      apiKey: "wrong",
      messages: [{ role: "user", content: "hi" }],
      signal: new AbortController().signal,
      onUpdate: () => {},
    }),
    /did not accept the app's key/,
  );
});
