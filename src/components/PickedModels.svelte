<script lang="ts">
  import { onMount } from "svelte";
  // The curated list ("Picked for you" tab).
  import DownloadControls from "./DownloadControls.svelte";
  import FitBadge, { FIT_TEXT } from "./FitBadge.svelte";
  import Icon from "./Icon.svelte";
  import Notice from "./Notice.svelte";
  import { errorText, getModels, listLocalModels } from "../lib/api";
  import { libraryVersion } from "../lib/downloads";
  import { approxWords, formatBytes, formatDateTime } from "../lib/format";
  import type { LocalModel, ModelList, ModelListItem } from "../lib/types";

  let list = $state<ModelList | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let search = $state("");
  let hideTooBig = $state(false);
  let local = $state<Record<string, LocalModel>>({});

  async function loadLocal() {
    try {
      const lib = await listLocalModels();
      local = Object.fromEntries(lib.models.map((m) => [m.id, m]));
    } catch {
      // Not critical here: buttons then show "Download" and Rust skips finished files.
    }
  }

  // Reload what is on disk when a download finishes, pauses or fails.
  $effect(() => {
    void $libraryVersion;
    void loadLocal();
  });

  async function load() {
    loading = true;
    error = null;
    try {
      list = await getModels();
    } catch (err) {
      error = errorText(err);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  function matches(m: ModelListItem, query: string): boolean {
    if (!query) return true;
    const text = [m.name, m.description ?? "", m.hf_repo, ...m.tags].join(" ").toLowerCase();
    return query
      .toLowerCase()
      .split(/\s+/)
      .every((word) => text.includes(word));
  }

  let shown = $derived(
    (list?.models ?? []).filter(
      (m) => matches(m, search.trim()) && !(hideTooBig && m.fit === "will_not_fit"),
    ),
  );
</script>

<div>
  {#if error}
    <Notice kind="error">
      <strong>{error}</strong>
    </Notice>
  {/if}

  {#if list}
    {#if list.needs_update}
      <Notice kind="warn">
        <strong>Please update the app.</strong>
        <span>The online list uses a newer format than this version of KuttyLM can read.</span>
      </Notice>
    {/if}
    {#if list.source === "cache" && list.fetched_at != null}
      <Notice kind="warn">
        <strong>Offline – list from {formatDateTime(list.fetched_at)}</strong>
        <span>Could not reach the online list. You see the last saved copy.</span>
      </Notice>
    {:else if list.source === "builtin" && list.offline}
      <Notice kind="warn">
        <strong>Offline</strong>
        <span>Could not reach the online list. You see the list that came with KuttyLM.</span>
      </Notice>
    {/if}

    <div class="tools">
      <input
        type="search"
        placeholder="Search models"
        aria-label="Search models"
        bind:value={search}
      />
      <label class="check">
        <input type="checkbox" bind:checked={hideTooBig} />
        Hide models that will not fit
      </label>
      <button class="btn refresh" onclick={load} disabled={loading}>
        <Icon name="refresh" size={16} />
        {loading ? "Loading…" : "Check again"}
      </button>
    </div>

    {#if shown.length === 0}
      <p class="muted empty">No models match your search.</p>
    {/if}

    <ul class="models">
      {#each shown as m (m.id)}
        <li class="model">
          <div class="info">
            <div class="title">
              <h2>{m.name}</h2>
              {#if m.trending}<span class="tag hot">Popular</span>{/if}
              {#if m.mmproj}<span class="tag">Can see images</span>{/if}
            </div>
            {#if m.description}<p class="desc">{m.description}</p>{/if}
            <p class="meta muted">
              <span>Download: {formatBytes(m.size_bytes)}</span>
              {#if m.context_length}
                <span title="How much text the model can keep in mind at once">
                  Remembers about {approxWords(m.context_length)}
                </span>
              {/if}
              {#if m.files.length > 1}<span>{m.files.length} files</span>{/if}
              {#if m.license}<span>Licence: {m.license}</span>{/if}
            </p>
            <p class="tech muted">
              {m.hf_repo}{m.quantization ? ` · ${m.quantization}` : ""}{m.params_b
                ? ` · ${m.params_b}B`
                : ""}
            </p>
          </div>
          <div class="side">
            <FitBadge fit={m.fit} />
            {#if m.fit !== "fits_well"}
              <p class="fit-help muted">{FIT_TEXT[m.fit].help}</p>
            {/if}
            <DownloadControls
              modelId={m.id}
              totalBytes={m.size_bytes}
              local={local[m.id]?.status ?? null}
              bytesOnDisk={local[m.id]?.bytes_on_disk ?? 0}
            />
          </div>
        </li>
      {/each}
    </ul>
  {:else if loading}
    <p class="muted">Loading the model list…</p>
  {/if}
</div>

<style>
  p {
    margin: 0;
  }

  .tools {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px 20px;
    margin-bottom: 16px;
  }

  input[type="search"] {
    flex: 1;
    min-width: 220px;
    max-width: 420px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  input[type="search"]:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .refresh {
    margin-left: auto;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    cursor: pointer;
  }

  .empty {
    padding: 20px 0;
  }

  .models {
    list-style: none;
    margin: 0;
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

  .tag.hot {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .desc {
    margin-bottom: 6px;
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
    justify-content: space-between;
    gap: 8px;
    width: 260px;
    flex-shrink: 0;
    text-align: right;
  }

  .fit-help {
    font-size: 0.8rem;
  }

  @media (max-width: 820px) {
    .model {
      flex-direction: column;
    }

    .side {
      width: auto;
      align-items: flex-start;
      text-align: left;
    }
  }
</style>
