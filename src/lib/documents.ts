// Documents for all screens: the list, indexing progress and the search model state.
import { writable } from "svelte/store";
import { getDocsStatus, listDocuments, onDocumentProgress } from "./api";
import { libraryVersion } from "./downloads";
import type { DocMeta, DocProgress, DocsStatus } from "./types";

export const documents = writable<DocMeta[]>([]);
export const docsStatus = writable<DocsStatus | null>(null);
/** Indexing progress per document id. */
export const docProgress = writable<Record<string, DocProgress>>({});

export async function refreshDocuments(): Promise<void> {
  try {
    const [status, list] = await Promise.all([getDocsStatus(), listDocuments()]);
    docsStatus.set(status);
    documents.set(list);
  } catch {
    // Keep the old list.
  }
}

let started = false;

export function startDocumentEvents(): void {
  if (started) return;
  started = true;
  void refreshDocuments();
  void onDocumentProgress((p) => {
    docProgress.update((all) => ({ ...all, [p.id]: p }));
    if (p.status !== "indexing" || p.done === 0) void refreshDocuments();
  });
  // The search model was downloaded or deleted.
  libraryVersion.subscribe(() => void refreshDocuments());
}
