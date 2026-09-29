<script lang="ts">
  // Disk space in three parts: KuttyLM models, other files, free space.
  import { formatBytes, percent } from "../lib/format";

  let { modelsBytes, freeBytes, totalBytes }: { modelsBytes: number; freeBytes: number; totalBytes: number } =
    $props();

  /** Below this, show a warning. */
  const LOW_SPACE = 10 * 1024 ** 3;

  let used = $derived(Math.max(0, totalBytes - freeBytes));
  let other = $derived(Math.max(0, used - modelsBytes));
  let modelsPct = $derived(percent(modelsBytes, totalBytes));
  let otherPct = $derived(percent(other, totalBytes));
  let low = $derived(freeBytes < LOW_SPACE);
</script>

<div
  class="bar"
  role="img"
  aria-label="Disk: {formatBytes(modelsBytes)} KuttyLM models, {formatBytes(other)} other files, {formatBytes(
    freeBytes,
  )} free"
>
  <!-- A small model folder still gets a visible sliver. -->
  <div class="seg models" style:width="max({modelsPct}%, {modelsBytes > 0 ? 4 : 0}px)"></div>
  <div class="seg other" class:low style:width="{otherPct}%"></div>
</div>

<ul class="legend">
  <li><span class="key models"></span>KuttyLM models <strong>{formatBytes(modelsBytes)}</strong></li>
  <li><span class="key other" class:low></span>Other files <strong>{formatBytes(other)}</strong></li>
  <li><span class="key free"></span>Free <strong>{formatBytes(freeBytes)}</strong></li>
  <li class="total">Disk size {formatBytes(totalBytes)} · {Math.round(percent(used, totalBytes))}% used</li>
</ul>

{#if low}
  <p class="warn">Your disk is almost full. Large models may not fit. Delete files you do not need to make space.</p>
{/if}

<style>
  .bar {
    display: flex;
    height: 12px;
    border-radius: 999px;
    background: var(--bar-bg);
    overflow: hidden;
    border: 1px solid var(--border);
  }

  .seg {
    height: 100%;
    transition: width 0.3s ease;
  }

  .seg.models,
  .key.models {
    background: var(--accent);
  }

  .seg.other,
  .key.other {
    background: var(--text-muted);
    opacity: 0.45;
  }

  .seg.other.low,
  .key.other.low {
    background: var(--warn);
    opacity: 0.7;
  }

  .legend {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 22px;
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .legend li {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .legend strong {
    color: var(--text);
    font-weight: 600;
  }

  .key {
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }

  .key.free {
    background: var(--bar-bg);
    border: 1px solid var(--border);
  }

  .total {
    margin-left: auto;
  }

  .warn {
    margin: 8px 0 0;
    color: var(--warn);
    font-size: 0.85rem;
  }
</style>
