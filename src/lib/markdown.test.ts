// Run: npm run test:ui
import { test } from "node:test";
import assert from "node:assert/strict";
import { parseInline, parseMarkdown, type Block, type Inline } from "./markdown.ts";

/** Plain text of inline parts, with simple marks, for short assertions. */
function flat(parts: Inline[]): string {
  return parts
    .map((p) => {
      switch (p.type) {
        case "text":
          return p.text;
        case "br":
          return "⏎";
        case "code":
          return `<code>${p.text}</code>`;
        case "link":
          return `<a ${p.href}>${flat(p.children)}</a>`;
        default:
          return `<${p.type}>${flat(p.children)}</${p.type}>`;
      }
    })
    .join("");
}

test("inline marks", () => {
  assert.equal(flat(parseInline("a **bold** and *it* and ~~no~~ and `x*y`")), "a <strong>bold</strong> and <em>it</em> and <del>no</del> and <code>x*y</code>");
  assert.equal(flat(parseInline("**bold *and it***")), "<strong>bold <em>and it</em></strong>");
  assert.equal(flat(parseInline("2 * 3 * 4")), "2 * 3 * 4");
  assert.equal(flat(parseInline("snake_case_name and __x__")), "snake_case_name and <strong>x</strong>");
  assert.equal(flat(parseInline("not \\*italic\\*")), "not *italic*");
  assert.equal(flat(parseInline("half **open")), "half **open");
  assert.equal(flat(parseInline("line one\nline two")), "line one⏎line two");
});

test("links are only http(s)", () => {
  assert.equal(flat(parseInline("[site](https://tryvom.com)")), "<a https://tryvom.com>site</a>");
  assert.equal(flat(parseInline("see https://example.com/a_b.")), "see <a https://example.com/a_b>https://example.com/a_b</a>.");
  // Unsafe links stay plain text.
  assert.equal(flat(parseInline("[x](javascript:alert(1))")), "[x](javascript:alert(1))");
  assert.equal(flat(parseInline("[x](file:///etc/passwd)")), "[x](file:///etc/passwd)");
});

test("headings, paragraphs, hr, quote", () => {
  const b = parseMarkdown("# Title\n\nSome text\nmore text\n\n---\n\n> quoted **line**\n> second");
  assert.deepEqual(
    b.map((x) => x.type),
    ["heading", "paragraph", "hr", "quote"],
  );
  assert.equal((b[0] as { level: number }).level, 1);
  const quote = b[3] as Extract<Block, { type: "quote" }>;
  assert.equal(quote.children[0].type, "paragraph");
});

test("code blocks, also unfinished", () => {
  const b = parseMarkdown("Try:\n\n```python\nprint('**not bold**')\n```\nafter");
  assert.deepEqual(b[1], { type: "code", lang: "python", text: "print('**not bold**')" });
  assert.equal(b[2].type, "paragraph");
  const open = parseMarkdown("```js\nlet a = 1;\nlet b");
  assert.deepEqual(open, [{ type: "code", lang: "js", text: "let a = 1;\nlet b" }]);
});

test("lists: bullets, numbers, nesting", () => {
  const b = parseMarkdown("1. First\n2. Second\n   - inner a\n   - inner b\n3. Third\n\nAfter");
  const list = b[0] as Extract<Block, { type: "list" }>;
  assert.equal(list.ordered, true);
  assert.equal(list.items.length, 3);
  const nested = list.items[1][1] as Extract<Block, { type: "list" }>;
  assert.equal(nested.type, "list");
  assert.equal(nested.ordered, false);
  assert.equal(nested.items.length, 2);
  assert.equal(b[1].type, "paragraph");

  const start = parseMarkdown("3. three\n4. four")[0] as Extract<Block, { type: "list" }>;
  assert.equal(start.start, 3);
});

test("tables", () => {
  const b = parseMarkdown("| Name | Size |\n|:---|---:|\n| **A** | 1 GB |\n| B | 2 GB |");
  const t = b[0] as Extract<Block, { type: "table" }>;
  assert.equal(t.type, "table");
  assert.deepEqual(t.align, ["left", "right"]);
  assert.equal(t.rows.length, 2);
  assert.equal(flat(t.rows[0][0]), "<strong>A</strong>");
});

test("plain text stays plain", () => {
  assert.deepEqual(parseMarkdown("Hello there."), [{ type: "paragraph", children: [{ type: "text", text: "Hello there." }] }]);
  assert.deepEqual(parseMarkdown(""), []);
});
