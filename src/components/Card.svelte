<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon, { type IconName } from "./Icon.svelte";

  let { title, icon, children }: { title: string; icon?: IconName; children: Snippet } = $props();
</script>

<section class="card">
  <h2>
    {#if icon}<Icon name={icon} size={18} />{/if}
    {title}
  </h2>
  {@render children()}
</section>

<style>
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    padding: 16px 18px;
  }

  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-muted);
    margin-bottom: 12px;
  }
</style>
