// A small Markdown reader for chat answers.
//
// It returns a tree (blocks and inline parts). Svelte components show the tree as normal
// elements, so model text is never inserted as HTML and cannot run code in the app.
// It also works on half-finished text while an answer is still arriving.
//
// Supported: headings, paragraphs, **bold**, *italic*, ~~strike~~, `code`, code blocks,
// lists (also nested and numbered), > quotes, tables, --- lines, links and plain URLs.

export type Inline =
  | { type: "text"; text: string }
  | { type: "strong" | "em" | "del"; children: Inline[] }
  | { type: "code"; text: string }
  | { type: "link"; href: string; children: Inline[] }
  | { type: "br" };

export type Align = "left" | "center" | "right" | null;

export type Block =
  | { type: "heading"; level: number; children: Inline[] }
  | { type: "paragraph"; children: Inline[] }
  | { type: "code"; lang: string; text: string }
  | { type: "list"; ordered: boolean; start: number; items: Block[][] }
  | { type: "quote"; children: Block[] }
  | { type: "table"; align: Align[]; header: Inline[][]; rows: Inline[][][] }
  | { type: "hr" };

// ---------- blocks ----------

const FENCE = /^ {0,3}(`{3,}|~{3,})\s*([\w+#.-]*)/;
const HEADING = /^ {0,3}(#{1,6})\s+(.*?)\s*#*\s*$/;
const HR = /^ {0,3}([-*_])(\s*\1){2,}\s*$/;
const QUOTE = /^ {0,3}>\s?(.*)$/;
const LIST_ITEM = /^(\s*)([-*+]|\d{1,9}[.)])\s+(.*)$/;
const TABLE_SEP = /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/;

function indentOf(line: string): number {
  return line.length - line.trimStart().length;
}

function isTableStart(lines: string[], i: number): boolean {
  return lines[i].includes("|") && i + 1 < lines.length && TABLE_SEP.test(lines[i + 1]) && lines[i + 1].includes("-");
}

function startsBlock(lines: string[], i: number): boolean {
  const line = lines[i];
  return (
    FENCE.test(line) || HEADING.test(line) || HR.test(line) || QUOTE.test(line) || LIST_ITEM.test(line) || isTableStart(lines, i)
  );
}

function splitRow(line: string): string[] {
  let s = line.trim();
  if (s.startsWith("|")) s = s.slice(1);
  if (s.endsWith("|") && !s.endsWith("\\|")) s = s.slice(0, -1);
  return s.split(/(?<!\\)\|/).map((c) => c.trim().replace(/\\\|/g, "|"));
}

export function parseMarkdown(text: string): Block[] {
  return parseBlocks(text.replace(/\r\n?/g, "\n").split("\n"));
}

function parseBlocks(lines: string[]): Block[] {
  const blocks: Block[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (!line.trim()) {
      i++;
      continue;
    }

    const fence = line.match(FENCE);
    if (fence) {
      const marker = fence[1];
      const body: string[] = [];
      i++;
      // An open fence (answer still arriving) runs to the end.
      while (i < lines.length && !lines[i].trim().startsWith(marker)) body.push(lines[i++]);
      i++;
      blocks.push({ type: "code", lang: fence[2] ?? "", text: body.join("\n") });
      continue;
    }

    const heading = line.match(HEADING);
    if (heading) {
      blocks.push({ type: "heading", level: heading[1].length, children: parseInline(heading[2]) });
      i++;
      continue;
    }

    if (HR.test(line)) {
      blocks.push({ type: "hr" });
      i++;
      continue;
    }

    if (QUOTE.test(line)) {
      const inner: string[] = [];
      while (i < lines.length && lines[i].trim() && QUOTE.test(lines[i])) inner.push(lines[i++].match(QUOTE)![1]);
      blocks.push({ type: "quote", children: parseBlocks(inner) });
      continue;
    }

    if (isTableStart(lines, i)) {
      const header = splitRow(lines[i]);
      const align: Align[] = splitRow(lines[i + 1]).map((c) =>
        c.startsWith(":") && c.endsWith(":") ? "center" : c.endsWith(":") ? "right" : c.startsWith(":") ? "left" : null,
      );
      i += 2;
      const rows: Inline[][][] = [];
      while (i < lines.length && lines[i].includes("|") && lines[i].trim()) {
        const cells = splitRow(lines[i++]);
        rows.push(header.map((_, c) => parseInline(cells[c] ?? "")));
      }
      blocks.push({ type: "table", align: header.map((_, c) => align[c] ?? null), header: header.map((h) => parseInline(h)), rows });
      continue;
    }

    const item = line.match(LIST_ITEM);
    if (item) {
      const [list, next] = parseList(lines, i);
      blocks.push(list);
      i = next;
      continue;
    }

    // Paragraph: until a blank line or another kind of block.
    const para: string[] = [line.trim()];
    i++;
    while (i < lines.length && lines[i].trim() && !startsBlock(lines, i)) para.push(lines[i++].trim());
    blocks.push({ type: "paragraph", children: parseInline(para.join("\n")) });
  }
  return blocks;
}

function parseList(lines: string[], start: number): [Block, number] {
  const first = lines[start].match(LIST_ITEM)!;
  const baseIndent = first[1].length;
  const ordered = /\d/.test(first[2]);
  const items: Block[][] = [];
  let i = start;

  while (i < lines.length) {
    const m = lines[i].match(LIST_ITEM);
    if (!m || m[1].length !== baseIndent || /\d/.test(m[2]) !== ordered) break;
    // The item's own text, then every following line that belongs to it.
    const content: string[] = [m[3]];
    const contentIndent = baseIndent + m[2].length + 1;
    i++;
    while (i < lines.length) {
      const line = lines[i];
      if (!line.trim()) {
        // A blank line ends the item unless indented text follows.
        const next = lines[i + 1];
        if (next !== undefined && next.trim() && indentOf(next) > baseIndent) {
          content.push("");
          i++;
          continue;
        }
        break;
      }
      const sub = line.match(LIST_ITEM);
      if (sub && sub[1].length <= baseIndent) break; // the next item of this (or a parent) list
      if (!sub && indentOf(line) <= baseIndent && startsBlock(lines, i)) break;
      content.push(line.slice(Math.min(indentOf(line), contentIndent)));
      i++;
    }
    items.push(parseBlocks(content));
  }
  const startNumber = ordered ? parseInt(first[2], 10) : 1;
  return [{ type: "list", ordered, start: Number.isFinite(startNumber) ? startNumber : 1, items }, i];
}

// ---------- inline ----------

const URL_START = /https?:\/\/[^\s<>()]+[^\s<>().,;:!?'"*_~]/y;

function safeHref(href: string): string | null {
  const url = href.trim();
  return /^https?:\/\//i.test(url) ? url : null;
}

/** Finds the closing marker that ends an emphasis started at `from`. */
function findClose(text: string, marker: string, from: number): number {
  let j = from;
  while ((j = text.indexOf(marker, j)) !== -1) {
    // The text inside must not start or end with a space, e.g. "2 * 3 * 4" is not italic.
    if (j > from && text[j - 1] !== " " && text[from] !== " ") {
      if (marker.length === 1 && text[j + 1] === marker) {
        j += 2; // part of a longer marker
        continue;
      }
      // In a longer run like "***", close with its last characters, so an inner
      // "*italic*" can use the first one: "**bold *and it***".
      while (marker.length === 2 && text[j + 2] === marker[0]) j++;
      return j;
    }
    j += marker.length;
  }
  return -1;
}

export function parseInline(text: string): Inline[] {
  const out: Inline[] = [];
  let buf = "";
  const flush = () => {
    if (buf) out.push({ type: "text", text: buf });
    buf = "";
  };

  let i = 0;
  while (i < text.length) {
    const ch = text[i];

    if (ch === "\\" && i + 1 < text.length && /[\\`*_~[\]()#+\-.!|>]/.test(text[i + 1])) {
      buf += text[i + 1];
      i += 2;
      continue;
    }

    if (ch === "\n") {
      flush();
      out.push({ type: "br" });
      i++;
      continue;
    }

    if (ch === "`") {
      const ticks = text.slice(i).match(/^`+/)![0];
      const end = text.indexOf(ticks, i + ticks.length);
      if (end !== -1) {
        flush();
        out.push({ type: "code", text: text.slice(i + ticks.length, end).trim() });
        i = end + ticks.length;
        continue;
      }
    }

    if (ch === "[") {
      const close = text.indexOf("](", i + 1);
      const end = close === -1 ? -1 : text.indexOf(")", close + 2);
      if (close !== -1 && end !== -1 && !text.slice(i + 1, close).includes("\n")) {
        const href = safeHref(text.slice(close + 2, end));
        if (href) {
          flush();
          out.push({ type: "link", href, children: parseInline(text.slice(i + 1, close)) });
          i = end + 1;
          continue;
        }
      }
    }

    if (ch === "h") {
      URL_START.lastIndex = i;
      const m = URL_START.exec(text);
      if (m && (i === 0 || /[\s(]/.test(text[i - 1]))) {
        flush();
        out.push({ type: "link", href: m[0], children: [{ type: "text", text: m[0] }] });
        i += m[0].length;
        continue;
      }
    }

    const two = text.slice(i, i + 2);
    if (two === "**" || two === "__" || two === "~~") {
      const end = findClose(text, two, i + 2);
      if (end !== -1) {
        flush();
        out.push({ type: two === "~~" ? "del" : "strong", children: parseInline(text.slice(i + 2, end)) });
        i = end + 2;
        continue;
      }
    }

    if ((ch === "*" || ch === "_") && text[i + 1] !== ch) {
      // "_" inside a word (snake_case) is not italic.
      const wordBefore = ch === "_" && i > 0 && /\w/.test(text[i - 1]);
      const end = wordBefore ? -1 : findClose(text, ch, i + 1);
      if (end !== -1 && !(ch === "_" && /\w/.test(text[end + 1] ?? ""))) {
        flush();
        out.push({ type: "em", children: parseInline(text.slice(i + 1, end)) });
        i = end + 1;
        continue;
      }
    }

    buf += ch;
    i++;
  }
  flush();
  return out;
}
