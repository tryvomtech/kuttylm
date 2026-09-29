<script lang="ts">
  import type { Snippet } from "svelte";

  let { kind = "info", children }: { kind?: "info" | "warn" | "error"; children: Snippet } = $props();
</script>

<div class="notice {kind}" role={kind === "error" ? "alert" : "status"}>
  {@render children()}
</div>

<style>
  .notice {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 14px;
    margin-bottom: 16px;
    border: 1px solid var(--border);
    border-left-width: 4px;
    border-radius: 8px;
    background: var(--surface);
    font-size: 0.92rem;
  }

  .info {
    border-left-color: var(--accent);
  }

  .warn {
    border-left-color: var(--warn);
  }

  .error {
    border-left-color: var(--bad);
    color: var(--bad);
  }
</style>
