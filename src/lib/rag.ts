// "Chat with documents": turns the found document parts into the question the model reads.
// The saved chat keeps only the user's own question; the excerpts are added per request.
import type { SearchHit, Source } from "./types";

/** How many document parts to give the model. Small context windows get fewer. */
export function excerptCount(contextSize: number | null): number {
  if (!contextSize) return 3;
  if (contextSize < 4096) return 2;
  if (contextSize < 8192) return 3;
  return 5;
}

export function sourceLabel(s: { doc_name: string; page: number | null }): string {
  return s.page ? `${s.doc_name}, page ${s.page}` : s.doc_name;
}

/** The question with the document parts in front of it. */
export function questionWithExcerpts(question: string, hits: SearchHit[]): string {
  if (hits.length === 0) return question;
  const parts = hits.map((h, i) => `[${i + 1}] From "${sourceLabel(h)}":\n${h.text.trim()}`);
  return [
    "Answer the question using these excerpts from my documents.",
    "If the excerpts do not contain the answer, say so, then answer from your own knowledge if you can.",
    "",
    parts.join("\n\n"),
    "",
    `Question: ${question}`,
  ].join("\n");
}

/** One entry per document and page, in the order found. */
export function sourcesOf(hits: SearchHit[]): Source[] {
  const seen = new Set<string>();
  const out: Source[] = [];
  for (const h of hits) {
    const key = `${h.doc_id}|${h.page ?? ""}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push({ doc_id: h.doc_id, doc_name: h.doc_name, page: h.page });
  }
  return out;
}
