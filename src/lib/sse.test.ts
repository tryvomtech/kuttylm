// Run: npm run test:ui
import { test } from "node:test";
import assert from "node:assert/strict";
import { SseParser } from "./sse.ts";

test("lines split across pieces", () => {
  const p = new SseParser();
  assert.deepEqual(p.push('data: {"a"'), []);
  assert.deepEqual(p.push(':1}\n\nda'), ['{"a":1}']);
  assert.deepEqual(p.push("ta: [DONE]\n"), ["[DONE]"]);
});

test("CRLF line ends and other fields", () => {
  const p = new SseParser();
  assert.deepEqual(p.push("event: x\r\ndata: one\r\n\r\n: comment\ndata:two\n"), ["one", "two"]);
});
