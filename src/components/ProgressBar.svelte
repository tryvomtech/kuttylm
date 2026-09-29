<script lang="ts">
  // Progress from 0 to 100. `muted` greys it out (for paused downloads).
  let { value, label, muted = false }: { value: number; label: string; muted?: boolean } = $props();
</script>

<div
  class="bar"
  role="progressbar"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={Math.round(value)}
>
  <div class="fill" class:muted style:width="{Math.max(0, Math.min(100, value))}%"></div>
</div>

<style>
  .bar {
    height: 8px;
    border-radius: 999px;
    background: var(--bar-bg);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    border-radius: 999px;
    transition: width 0.25s linear;
  }

  .fill.muted {
    background: var(--text-muted);
  }
</style>
