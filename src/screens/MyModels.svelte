<script lang="ts">
  import Card from "../components/Card.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import DownloadControls from "../components/DownloadControls.svelte";
  import Icon from "../components/Icon.svelte";
  import StorageBar from "../components/StorageBar.svelte";
  import Notice from "../components/Notice.svelte";
  import RunControls from "../components/RunControls.svelte";
  import { deleteModel, errorText, listLocalModels } from "../lib/api";
  import { downloads, isActive, libraryVersion } from "../lib/downloads";
  import { engine } from "../lib/engine";
  import { formatBytes, formatDateTime } from "../lib/format";
  import { currentScreen } from "../lib/stores";
  import type { Library, LocalModel } from "../lib/types";

  let library = $state<Library | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let toDelete = $state<LocalModel | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      library = await listLocalModels();
    } catch (err) {
      error = errorText(err);
    } finally {
      loading = false;
    }
  }

  // Load now, and again when a download finishes, pauses or is cancelled.
  $effect(() => {
    void $libraryVersion;
    void load();
  });

  // While a download runs, the sizes change. Refresh the storage numbers every few seconds.
  let anyActive = $derived(Object.values($downloads).some(isActive));
  $effect(() => {
    if (!anyActive) return;
    const timer = setInterval(load, 3000);
    return () => clearInterval(timer);
  });

  async function confirmDelete() {
    const model = toDelete;
    toDelete = null;
    if (!model) return;
    error = null;
    try {
      await deleteModel(model.id);
    } catch (err) {
      error = errorText(err);
    }
    await load();
  }

  let disk = $derived(library?.disk ?? null);
</script>

<div class="page">
  <header>
    <div>
      <h1>My models</h1>
      <p class="muted">Models saved on this computer.</p>
    </div>
    <button class="btn" onclick={load} disabled={loading}>
      <Icon name="refresh" size={16} />
      Check again
    </button>
  </header>

  {#if error}
    <Notice kind="error"><strong>{error}</strong></Notice>
  {/if}

  {#if library}
    <Card title="Storage" icon="memory">
      <div class="storage">
        <div class="stat">
          <span class="muted">My models use</span>
          <strong>{formatBytes(library.used_bytes)}</strong>
          <span class="muted small">
            {library.models.length}
            {library.models.length === 1 ? "model" : "models"}
          </span>
        </div>
        <div class="stat">
          <span class="muted">Free disk space</span>
          <strong>{disk ? formatBytes(disk.free_bytes) : "Unknown"}</strong>
          {#if disk}
            <span class="muted small">of {formatBytes(disk.total_bytes)}</span>
          {/if}
        </div>
      </div>
      {#if disk}
        <StorageBar modelsBytes={library.used_bytes} freeBytes={disk.free_bytes} totalBytes={disk.total_bytes} />
      {/if}
      <p class="folder muted small">Folder: <span class="path">{library.models_dir}</span></p>
    </Card>

    {#if library.models.length === 0}
      <div class="empty">
        <p>You have no models yet.</p>
        <button class="btn btn-primary" onclick={() => currentScreen.set("browse")}>Find models</button>
      </div>
    {:else}
      <ul class="models">
        {#each library.models as m (m.id)}
          <li class="model">
            <div class="info">
              <div class="title">
                <h2>{m.name}</h2>
                {#if m.tags.includes("embedding")}
                  <span class="tag">For Documents</span>
                {/if}
                {#if m.status === "ready"}
                  <span class="tag ok">Ready</span>
                {:else}
                  <span class="tag">Not finished</span>
                {/if}
              </div>
              <p class="meta muted">
                {#if m.status === "ready"}
                  <span>{formatBytes(m.bytes_on_disk)}</span>
                  <span>Downloaded {formatDateTime(m.saved_at)}</span>
                {:else}
                  <span>{formatBytes(m.bytes_on_disk)} of {formatBytes(m.total_bytes)}</span>
                  <span>Started {formatDateTime(m.saved_at)}</span>
                {/if}
              </p>
              {#if m.tags.includes("embedding")}
                <p class="meta muted">Finds the right parts of your documents. It cannot chat.</p>
              {/if}
              <p class="tech muted">{m.hf_repo}{m.quantization ? ` · ${m.quantization}` : ""}</p>
            </div>
            <div class="side">
              {#if m.status === "ready"}
                {#if m.tags.includes("embedding")}
                  <button class="btn" onclick={() => currentScreen.set("documents")}>Open Documents</button>
                {:else}
                  <RunControls modelId={m.id} />
                {/if}
                <button class="btn btn-quiet" onclick={() => (toDelete = m)}>Delete</button>
              {:else}
                <DownloadControls
                  modelId={m.id}
                  totalBytes={m.total_bytes}
                  local={m.status}
                  bytesOnDisk={m.bytes_on_disk}
                />
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  {:else if loading}
    <p class="muted">Loading…</p>
  {/if}
</div>

<ConfirmDialog
  open={toDelete !== null}
  title="Delete {toDelete?.name ?? 'this model'}?"
  message="{toDelete && $engine.model_id === toDelete.id && $engine.state !== 'stopped'
    ? 'The model will be stopped first. '
    : ''}{toDelete?.tags.includes('embedding') ? 'Documents cannot be used until you download it again. ' : ''}The model files ({formatBytes(toDelete?.bytes_on_disk ?? 0)}) will be removed from this computer. You can download it again later."
  confirmLabel="Delete"
  danger
  onconfirm={confirmDelete}
  oncancel={() => (toDelete = null)}
/>

<style>
  .page {
    padding: 28px 32px;
    max-width: 1100px;
  }

  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 20px;
  }

  h1 {
    font-size: 1.5rem;
    margin-bottom: 4px;
  }

  p {
    margin: 0;
  }

  .storage {
    display: flex;
    flex-wrap: wrap;
    gap: 12px 48px;
    margin-bottom: 12px;
  }

  .stat {
    display: flex;
    flex-direction: column;
  }

  .stat strong {
    font-size: 1.5rem;
  }

  .small {
    font-size: 0.85rem;
  }

  .folder {
    margin-top: 10px;
  }

  .path {
    font-family: ui-monospace, "Ubuntu Mono", monospace;
    user-select: all;
    overflow-wrap: anywhere;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
    padding: 24px 0;
  }

  .models {
    list-style: none;
    margin: 16px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .model {
    display: flex;
    gap: 20px;
    justify-content: space-between;
    padding: 16px 18px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
  }

  .info {
    min-width: 0;
    flex: 1;
  }

  .title {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 4px;
  }

  h2 {
    font-size: 1.05rem;
  }

  .tag {
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

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
    font-size: 0.88rem;
  }

  .tech {
    margin-top: 4px;
    font-size: 0.78rem;
    overflow-wrap: anywhere;
  }

  .side {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    justify-content: center;
    gap: 8px;
    width: 280px;
    flex-shrink: 0;
  }

  .btn-quiet {
    border-color: transparent;
    background: none;
    color: var(--text-muted);
    font-size: 0.85rem;
    padding: 4px 8px;
  }

  @media (max-width: 820px) {
    .model {
      flex-direction: column;
    }

    .side {
      width: auto;
      align-items: flex-start;
    }
  }
</style>
