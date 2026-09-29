<script lang="ts">
  // A thin bar that shows how much of something is used.
  let { value, label }: { value: number; label: string } = $props();
</script>

<div
  class="bar"
  role="meter"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={Math.round(value)}
>
  <div class="fill" class:high={value >= 85} style:width="{value}%"></div>
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
    transition: width 0.3s ease;
  }

  .fill.high {
    background: var(--warn);
  }
</style>
