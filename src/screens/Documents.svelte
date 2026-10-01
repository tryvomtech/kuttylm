<script lang="ts">
  import Card from "../components/Card.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import DownloadControls from "../components/DownloadControls.svelte";
  import Icon from "../components/Icon.svelte";
  import Notice from "../components/Notice.svelte";
  import ProgressBar from "../components/ProgressBar.svelte";
  import { addDocument, deleteDocument, errorText } from "../lib/api";
  import { docProgress, documents, docsStatus, refreshDocuments } from "../lib/documents";
  import { formatBytes, formatDateTime, percent } from "../lib/format";
  import { currentScreen } from "../lib/stores";
  import type { DocKind, DocMeta } from "../lib/types";

  const ACCEPT = ".pdf,.docx,.txt,.md,.markdown";
  const kindLabel: Record<DocKind, string> = { pdf: "PDF", docx: "Word", text: "Text", markdown: "Markdown" };

  let fileInput: HTMLInputElement | undefined = $state();
  let errors = $state<string[]>([]);
  let uploading = $state(0);
  let dragging = $state(false);
  let toDelete = $state<DocMeta | null>(null);

  $effect(() => {
    void refreshDocuments();
  });

  let ready = $derived($docsStatus?.model_ready ?? false);

  async function addFiles(files: FileList | File[]) {
    errors = [];
    for (const file of Array.from(files)) {
      uploading += 1;
      try {
        await addDocument(file);
      } catch (err) {
        errors = [...errors, `${file.name}: ${errorText(err)}`];
      } finally {
        uploading -= 1;
      }
    }
    await refreshDocuments();
  }

  function onPick(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    if (input.files?.length) void addFiles(input.files);
    input.value = "";
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    if (!ready) return;
    const files = e.dataTransfer?.files;
    if (files?.length) void addFiles(files);
  }

  async function confirmDelete() {
    const doc = toDelete;
    toDelete = null;
    if (!doc) return;
    try {
      await deleteDocument(doc.id);
    } catch (err) {
      errors = [errorText(err)];
    }
    await refreshDocuments();
  }

  function details(d: DocMeta): string {
    const parts = [kindLabel[d.kind], formatBytes(d.size_bytes)];
    if (d.pages) parts.push(`${d.pages} ${d.pages === 1 ? "page" : "pages"}`);
    parts.push(`Added ${formatDateTime(d.created_at)}`);
    return parts.join(" · ");
  }
</script>

<div
  class="page"
  role="region"
  aria-label="Documents"
  ondragover={(e) => {
    e.preventDefault();
    if (ready) dragging = true;
  }}
  ondragleave={(e) => {
    if (e.currentTarget === e.target) dragging = false;
  }}
  ondrop={onDrop}
>
  <header>
    <div>
      <h1>Documents</h1>
      <p class="muted">Add your files, then ask questions about them in Chat.</p>
    </div>
    {#if ready}
      <button class="btn btn-primary" onclick={() => fileInput?.click()}>
        <Icon name="plus" size={16} /> Add documents
      </button>
    {/if}
    <input bind:this={fileInput} type="file" accept={ACCEPT} multiple hidden onchange={onPick} />
  </header>

  {#if $docsStatus && !ready}
    <Card title="First, download the document search model" icon="file">
      <div class="setup">
        <p>
          KuttyLM needs a small extra model ({formatBytes($docsStatus.model_size_bytes)}) to find the right parts
          of your documents. You download it only once. It works offline, and your documents never leave this
          computer.
        </p>
        <div class="download">
          <DownloadControls modelId={$docsStatus.model_id} totalBytes={$docsStatus.model_size_bytes} local={null} />
        </div>
      </div>
    </Card>
  {/if}

  {#if ready}
    <button class="drop" class:dragging onclick={() => fileInput?.click()}>
      <Icon name="file" size={28} />
      <strong>Drop files here, or click to choose</strong>
      <span class="muted">PDF, Word (.docx), text (.txt) and Markdown (.md) · up to 50 MB each</span>
    </button>
  {/if}

  {#if uploading > 0}
    <p class="muted">Reading {uploading} {uploading === 1 ? "file" : "files"}…</p>
  {/if}

  {#each errors as message (message)}
    <Notice kind="error"><strong>{message}</strong></Notice>
  {/each}

  {#if $documents.length > 0}
    <ul class="docs">
      {#each $documents as d (d.id)}
        {@const p = $docProgress[d.id]}
        <li class="doc">
          <div class="info">
            <div class="title">
              <h2 title={d.name}>{d.name}</h2>
              {#if d.status === "ready"}
                <span class="tag ok">Ready</span>
              {:else if d.status === "indexing"}
                <span class="tag">Preparing…</span>
              {:else}
                <span class="tag bad">Failed</span>
              {/if}
            </div>
            <p class="meta muted">{details(d)}</p>
            {#if d.status === "indexing"}
              <div class="progress">
                <ProgressBar value={p && p.total ? percent(p.done, p.total) : 0} label="Preparing the document" />
                <span class="muted small">
                  {p && p.total ? `${p.done} of ${p.total} parts` : "Reading the text…"}
                </span>
              </div>
            {:else if d.status === "failed" && d.error}
              <p class="error">{d.error}</p>
            {/if}
          </div>
          <button class="btn btn-quiet" onclick={() => (toDelete = d)}>
            {d.status === "indexing" ? "Stop and delete" : "Delete"}
          </button>
        </li>
      {/each}
    </ul>
    {#if $documents.some((d) => d.status === "ready")}
      <p class="hint muted">
        To use a document: open <button class="link" onclick={() => currentScreen.set("chat")}>Chat</button>,
        click <strong>Documents</strong> under the message box and choose it.
      </p>
    {/if}
  {:else if ready}
    <p class="muted empty">No documents yet.</p>
  {/if}
</div>

<ConfirmDialog
  open={toDelete !== null}
  title="Delete {toDelete?.name ?? 'this document'}?"
  message="KuttyLM forgets this document. Your original file is not touched."
  confirmLabel="Delete"
  danger
  onconfirm={confirmDelete}
  oncancel={() => (toDelete = null)}
/>

<style>
  .page {
    padding: 28px 32px;
    max-width: 1000px;
    min-height: 100%;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 6px;
  }

  h1 {
    font-size: 1.5rem;
    margin-bottom: 4px;
  }

  p {
    margin: 0;
  }

  .setup {
    display: flex;
    gap: 20px;
    align-items: center;
  }

  .setup p {
    flex: 1;
  }

  .download {
    width: 280px;
    flex-shrink: 0;
  }

  .drop {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 26px 16px;
    border: 2px dashed var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }

  .drop:hover,
  .drop.dragging {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .drop .muted {
    font-size: 0.85rem;
  }

  .docs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .doc {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 18px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .info {
    flex: 1;
    min-width: 0;
  }

  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 2px;
  }

  h2 {
    font-size: 1rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag {
    flex-shrink: 0;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 600;
  }

  .tag.ok {
    color: var(--good);
  }

  .tag.bad {
    color: var(--bad);
  }

  .meta,
  .small {
    font-size: 0.85rem;
  }

  .progress {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 8px;
    max-width: 420px;
  }

  .error {
    margin-top: 6px;
    color: var(--bad);
    font-size: 0.88rem;
  }

  .btn-quiet {
    border-color: transparent;
    background: none;
    color: var(--text-muted);
    font-size: 0.85rem;
    padding: 4px 8px;
  }

  .hint {
    font-size: 0.88rem;
  }

  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }

  .empty {
    padding: 8px 0;
  }

  @media (max-width: 820px) {
    .setup {
      flex-direction: column;
      align-items: stretch;
    }

    .download {
      width: auto;
    }
  }
</style>
