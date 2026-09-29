<script lang="ts">
  // "Search Hugging Face" tab: search, sort and size filters, pages of 20 results.
  import { onMount } from "svelte";
  import HfRepoPanel from "./HfRepoPanel.svelte";
  import Notice from "./Notice.svelte";
  import { errorText, hfSearch, listLocalModels } from "../lib/api";
  import { libraryVersion } from "../lib/downloads";
  import { formatCount } from "../lib/format";
  import type { HfRepo, HfSize, HfSort, LocalModel } from "../lib/types";

  let query = $state("");
  let sort = $state<HfSort>("downloads");
  let size = $state<HfSize>("any");
  let onlyFits = $state(true);

  let results = $state<HfRepo[]>([]);
  let cursor = $state<string | null>(null);
  let loading = $state(false);
  let searched = $state(false);
  let error = $state<string | null>(null);
  let open = $state<Record<string, boolean>>({});
  let local = $state<LocalModel[]>([]);

  $effect(() => {
    void $libraryVersion;
    listLocalModels()
      .then((lib) => (local = lib.models))
      .catch(() => {});
  });

  /** New search (`more` = false) or the next page (`more` = true). */
  async function run(more = false) {
    loading = true;
    error = null;
    try {
      const page = await hfSearch(query, sort, size, more ? cursor : null);
      results = more ? [...results, ...page.results] : page.results;
      cursor = page.next_cursor;
      if (!more) open = {};
      searched = true;
    } catch (err) {
      error = errorText(err);
    } finally {
      loading = false;
    }
  }

  // Popular GGUF models when the tab opens (about 4 KB).
  onMount(() => void run());

  function kind(tag: string | null): string | null {
    if (tag === "text-generation") return "Chat";
    if (tag === "image-text-to-text") return "Can see images";
    return null;
  }

  function daysAgo(iso: string | null): string | null {
    if (!iso) return null;
    const days = Math.floor((Date.now() - Date.parse(iso)) / 86_400_000);
    if (!Number.isFinite(days)) return null;
    if (days < 1) return "today";
    if (days < 60) return `${days} ${days === 1 ? "day" : "days"} ago`;
    const months = Math.floor(days / 30);
    return months < 24 ? `${months} months ago` : `${Math.floor(days / 365)} years ago`;
  }
</script>

<Notice>
  <span>These models come from Hugging Face. KuttyLM has not checked them. Models that need a login are not shown.</span>
</Notice>

<form
  class="tools"
  onsubmit={(e) => {
    e.preventDefault();
    void run();
  }}
>
  <div class="row">
    <input type="search" placeholder="Search, for example: qwen, llama 3b, coder" aria-label="Search Hugging Face" bind:value={query} />
    <button class="btn btn-primary" type="submit" disabled={loading}>Search</button>
  </div>
  <div class="row">
  <label class="select">
    <span>Sort</span>
    <select bind:value={sort} onchange={() => run()}>
      <option value="downloads">Most downloaded</option>
      <option value="likes">Most liked</option>
      <option value="trending">Trending</option>
      <option value="updated">Recently updated</option>
    </select>
  </label>
  <label class="select">
    <span>Model size</span>
    <select bind:value={size} onchange={() => run()}>
      <option value="any">Any size</option>
      <option value="tiny">Tiny (under 1B)</option>
      <option value="small">Small (1B – 4B)</option>
      <option value="medium">Medium (4B – 14B)</option>
      <option value="large">Large (14B and more)</option>
    </select>
  </label>
  <label class="check">
    <input type="checkbox" bind:checked={onlyFits} />
    Only versions that fit
  </label>
  </div>
</form>

{#if error}
  <Notice kind="error"><strong>{error}</strong></Notice>
{/if}

{#if searched && results.length === 0 && !loading && !error}
  <p class="muted empty">No models found. Try other words or another size.</p>
{/if}

<ul class="repos">
  {#each results as r (r.id)}
    <li class="repo">
      <div class="head">
        <div class="info">
          <h2>{r.id.split("/")[1]}</h2>
          <p class="muted small">
            <span>by {r.id.split("/")[0]}</span>
            <span>↓ {formatCount(r.downloads)}</span>
            <span>♥ {formatCount(r.likes)}</span>
            {#if kind(r.pipeline_tag)}<span>{kind(r.pipeline_tag)}</span>{/if}
            {#if r.license}<span>Licence: {r.license}</span>{/if}
            {#if daysAgo(r.last_modified)}<span>Updated {daysAgo(r.last_modified)}</span>{/if}
          </p>
        </div>
        <button class="btn" aria-expanded={!!open[r.id]} onclick={() => (open[r.id] = !open[r.id])}>
          {open[r.id] ? "Hide versions" : "Show versions"}
        </button>
      </div>
      {#if open[r.id]}
        <HfRepoPanel repo={r.id} {local} {onlyFits} />
      {/if}
    </li>
  {/each}
</ul>

{#if loading}
  <p class="muted empty">Searching…</p>
{:else if cursor}
  <button class="btn more" onclick={() => run(true)}>Show more</button>
{/if}

<style>
  .tools {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 16px;
  }

  .row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px 18px;
  }

  input[type="search"] {
    flex: 1;
    min-width: 220px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  input[type="search"]:focus,
  select:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .select {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
    font-size: 0.9rem;
  }

  select {
    padding: 7px 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    cursor: pointer;
  }

  .empty {
    padding: 16px 0;
  }

  .repos {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .repo {
    padding: 14px 18px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .info {
    min-width: 0;
  }

  h2 {
    font-size: 1.02rem;
    margin: 0 0 2px;
    overflow-wrap: anywhere;
  }

  .small {
    font-size: 0.85rem;
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 2px 14px;
  }

  .more {
    margin: 16px auto 0;
    display: flex;
  }
</style>
