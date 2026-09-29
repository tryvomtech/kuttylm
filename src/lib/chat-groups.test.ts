// Run: npm run test:ui
import { test } from "node:test";
import assert from "node:assert/strict";
import { groupChats } from "./chat-groups.ts";

const now = new Date(2026, 8, 29, 15, 0); // 29 Sept 2026, 3 pm
const at = (y: number, m: number, d: number, h = 12) => new Date(y, m, d, h).getTime() / 1000;
const chat = (id: string, t: number) => ({ id, title: id, created_at: t, updated_at: t, model_name: null });

test("chats are grouped by date, newest first", () => {
  const groups = groupChats(
    [
      chat("old", at(2026, 6, 1)),
      chat("today-early", at(2026, 8, 29, 1)),
      chat("yesterday", at(2026, 8, 28, 23)),
      chat("week", at(2026, 8, 24)),
      chat("month", at(2026, 8, 5)),
      chat("today-late", at(2026, 8, 29, 14)),
    ],
    now,
  );
  assert.deepEqual(
    groups.map((g) => [g.label, g.chats.map((c) => c.id)]),
    [
      ["Today", ["today-late", "today-early"]],
      ["Yesterday", ["yesterday"]],
      ["Previous 7 days", ["week"]],
      ["Previous 30 days", ["month"]],
      [new Date(2026, 6, 1).toLocaleDateString(undefined, { month: "long", year: "numeric" }), ["old"]],
    ],
  );
});

test("no chats, no groups", () => {
  assert.deepEqual(groupChats([], now), []);
});
