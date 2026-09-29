<script lang="ts">
  import HfSearch from "../components/HfSearch.svelte";
  import PickedModels from "../components/PickedModels.svelte";

  type Tab = "picked" | "hf";
  let tab = $state<Tab>("picked");
</script>

<div class="page">
  <header>
    <h1>Find models</h1>
    <p class="muted">Each model shows if it fits this computer.</p>
  </header>

  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={tab === "picked"} class:active={tab === "picked"} onclick={() => (tab = "picked")}>
      Picked for you
    </button>
    <button role="tab" aria-selected={tab === "hf"} class:active={tab === "hf"} onclick={() => (tab = "hf")}>
      Search Hugging Face
    </button>
  </div>

  <div role="tabpanel">
    {#if tab === "picked"}
      <PickedModels />
    {:else}
      <HfSearch />
    {/if}
  </div>
</div>

<style>
  .page {
    padding: 28px 32px;
    max-width: 1100px;
  }

  header {
    margin-bottom: 16px;
  }

  h1 {
    font-size: 1.5rem;
    margin-bottom: 4px;
  }

  header p {
    margin: 0;
  }

  .tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 18px;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    padding: 8px 14px;
    border: none;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--text-muted);
    font-weight: 600;
    margin-bottom: -1px;
  }

  .tabs button:hover {
    color: var(--text);
  }

  .tabs button.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
</style>
