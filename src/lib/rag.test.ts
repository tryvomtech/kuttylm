import assert from "node:assert/strict";
import { test } from "node:test";
import { excerptCount, questionWithExcerpts, sourceLabel, sourcesOf } from "./rag.ts";
import type { SearchHit } from "./types";

const hit = (doc_id: string, page: number | null, text = "Some text."): SearchHit => ({
  doc_id,
  doc_name: `${doc_id}.pdf`,
  page,
  text,
  score: 0.5,
});

test("fewer excerpts for small models", () => {
  assert.equal(excerptCount(2048), 2);
  assert.equal(excerptCount(4096), 3);
  assert.equal(excerptCount(8192), 5);
  assert.equal(excerptCount(null), 3);
});

test("question with excerpts", () => {
  assert.equal(questionWithExcerpts("Why?", []), "Why?");
  const q = questionWithExcerpts("What is the fee?", [hit("a", 3, " The fee is 10. "), hit("b", null)]);
  assert.match(q, /\[1\] From "a\.pdf, page 3":\nThe fee is 10\./);
  assert.match(q, /\[2\] From "b\.pdf":\nSome text\./);
  assert.ok(q.endsWith("Question: What is the fee?"));
});

test("sources are listed once per page", () => {
  const s = sourcesOf([hit("a", 1), hit("a", 1), hit("a", 2), hit("b", null)]);
  assert.deepEqual(
    s.map((x) => sourceLabel(x)),
    ["a.pdf, page 1", "a.pdf, page 2", "b.pdf"],
  );
});
