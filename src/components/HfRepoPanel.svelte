<script lang="ts">
  // The downloadable versions of one Hugging Face repository.
  import { onMount } from "svelte";
  import DownloadControls from "./DownloadControls.svelte";
  import FitBadge from "./FitBadge.svelte";
  import Notice from "./Notice.svelte";
  import { errorText, hfRepoFiles } from "../lib/api";
  import { approxWords, formatBytes } from "../lib/format";
  import type { HfRepoDetail, HfVariant, LocalModel } from "../lib/types";

  let { repo, local, onlyFits }: { repo: string; local: LocalModel[]; onlyFits: boolean } = $props();

  let detail = $state<HfRepoDetail | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      detail = await hfRepoFiles(repo);
    } catch (err) {
      error = errorText(err);
    }
  });

  /** The same file may already be on disk, for example from "Picked for you". */
  function localCopy(v: HfVariant): LocalModel | undefined {
    return local.find((m) => m.id === v.id || (m.hf_repo === v.hf_repo && m.files[0]?.name === v.files[0]?.name));
  }

  // The recommended version first, then the rest by size.
  let shown = $derived(
    (detail?.variants ?? [])
      .filter((v) => !onlyFits || v.fit !== "will_not_fit")
      .sort((a, b) => Number(b.recommended) - Number(a.recommended) || a.size_bytes - b.size_bytes),
  );
  let hidden = $derived((detail?.variants.length ?? 0) - shown.length);
</script>

<div class="panel">
  {#if error}
    <Notice kind="error"><strong>{error}</strong></Notice>
  {:else if !detail}
    <p class="muted small">Loading the file list…</p>
  {:else}
    <p class="facts muted small">
      {#if detail.params_b}<span>{detail.params_b}B parameters</span>{/if}
      {#if detail.context_length}<span>Remembers about {approxWords(Math.min(detail.context_length, 8192))}</span>{/if}
      {#if detail.vision}<span>Can see images</span>{/if}
      {#if detail.license}<span>Licence: {detail.license}</span>{/if}
    </p>

    {#if detail.variants.length === 0}
      <p class="muted small">No downloadable versions were found in this model.</p>
    {:else}
      <ul class="variants">
        {#each shown as v (v.id)}
          {@const copy = localCopy(v)}
          <li class="variant">
            <div class="vinfo">
              <div class="vtitle">
                <strong>{v.quantization ?? v.files[0].name}</strong>
                {#if v.recommended}<span class="tag rec">Recommended</span>{/if}
              </div>
              <span class="muted small">
                {formatBytes(v.size_bytes)}{v.files.length > 1 ? ` · ${v.files.length} files` : ""}{v.mmproj
                  ? " · with image support"
                  : ""}
              </span>
            </div>
            <FitBadge fit={v.fit} />
            <div class="vdl">
              <DownloadControls
                modelId={copy?.id ?? v.id}
                totalBytes={v.size_bytes}
                local={copy?.status ?? null}
                bytesOnDisk={copy?.bytes_on_disk ?? 0}
              />
            </div>
          </li>
        {/each}
      </ul>
      {#if hidden > 0}
        <p class="muted small">{hidden} bigger {hidden === 1 ? "version is" : "versions are"} hidden because {hidden === 1 ? "it does" : "they do"} not fit this computer.</p>
      {/if}
    {/if}
    {#if detail.has_folder_files}
      <p class="muted small">Some very large versions are stored in folders. KuttyLM cannot download those yet.</p>
    {/if}
  {/if}
</div>

<style>
  .panel {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  p {
    margin: 0;
  }

  .small {
    font-size: 0.85rem;
  }

  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
  }

  .variants {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .variant {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto 260px;
    align-items: center;
    gap: 16px;
    padding: 10px 0;
  }

  .variant + .variant {
    border-top: 1px dashed var(--border);
  }

  .vinfo {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .vtitle {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tag.rec {
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 0.75rem;
    font-weight: 600;
  }

  .vdl {
    display: flex;
    justify-content: flex-end;
  }

  @media (max-width: 820px) {
    .variant {
      grid-template-columns: minmax(0, 1fr) auto;
    }

    .vdl {
      grid-column: 1 / -1;
      justify-content: flex-start;
    }
  }
</style>
