// Run: npm run test:ui
// The last test needs a running llama-server: set KUTTYLM_TEST_CHAT_URL and KUTTYLM_TEST_CHAT_KEY.
import { test } from "node:test";
import assert from "node:assert/strict";
import { answerReserve, contextSize, countTokens, fitMessages, loadIntoModel } from "./context.ts";
import type { ChatMessage } from "./chat.ts";

const msg = (role: "user" | "assistant", words: number): ChatMessage => ({ role, content: "w ".repeat(words).trim() });
/** Fake counter: 1 token per word plus 4 per message. */
const fakeCount = async (m: ChatMessage[]) => m.reduce((n, x) => n + x.content.split(" ").length + 4, 0);

test("a chat that fits stays complete", async () => {
  const chat = [msg("user", 10), msg("assistant", 20), msg("user", 5)];
  const r = await fitMessages(chat, 100, fakeCount);
  assert.equal(r.dropped, 0);
  assert.equal(r.messages.length, 3);
  assert.equal(r.tokens, 47);
});

test("oldest question-and-answer pairs are left out first", async () => {
  const chat = [msg("user", 50), msg("assistant", 50), msg("user", 10), msg("assistant", 10), msg("user", 5)];
  const r = await fitMessages(chat, 50, fakeCount);
  assert.equal(r.dropped, 2);
  assert.equal(r.messages[0].role, "user");
  assert.ok(r.tokens <= 50);
});

test("the newest message is always kept", async () => {
  const r = await fitMessages([msg("user", 10), msg("assistant", 10), msg("user", 500)], 50, fakeCount);
  assert.equal(r.messages.length, 1);
  assert.equal(r.dropped, 2);
});

test("answer reserve", () => {
  assert.equal(answerReserve(2048), 512);
  assert.equal(answerReserve(8192), 1024);
});

const url = process.env.KUTTYLM_TEST_CHAT_URL;
test("real context numbers from llama-server", { skip: !url }, async () => {
  const o = { baseUrl: url!, apiKey: process.env.KUTTYLM_TEST_CHAT_KEY ?? null };
  const size = await contextSize(o);
  const chat: ChatMessage[] = [
    { role: "user", content: "Hello, who are you?" },
    { role: "assistant", content: "I am a helpful assistant." },
  ];
  const tokens = await countTokens(o, chat);
  await loadIntoModel(o, chat);
  console.log(`context size ${size}, chat uses ${tokens} tokens`);
  assert.ok(size && size > 0);
  assert.ok(tokens > 5 && tokens < 200);
});
