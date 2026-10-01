<script lang="ts">
  // "Documents" button under the message box: choose which documents the chat may use.
  import Icon from "./Icon.svelte";
  import { chatDocs } from "../lib/chat-session";
  import { documents, docsStatus } from "../lib/documents";
  import { currentScreen } from "../lib/stores";

  let open = $state(false);
  let box: HTMLElement | undefined = $state();

  let ready = $derived($documents.filter((d) => d.status === "ready"));
  let chosen = $derived($chatDocs.filter((id) => ready.some((d) => d.id === id)));
  let modelReady = $derived($docsStatus?.model_ready ?? false);

  function toggle(id: string) {
    chatDocs.update((ids) => (ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id]));
  }

  function goToDocuments() {
    open = false;
    currentScreen.set("documents");
  }

  // Close when clicking outside or pressing Escape.
  $effect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      if (box && !box.contains(e.target as Node)) open = false;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") open = false;
    };
    document.addEventListener("mousedown", onClick);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onClick);
      document.removeEventListener("keydown", onKey);
    };
  });
</script>

<div class="picker" bind:this={box}>
  <button
    type="button"
    class="toggle"
    class:on={chosen.length > 0}
    aria-expanded={open}
    onclick={() => (open = !open)}
    title="Choose documents for this chat"
  >
    <Icon name="file" size={15} />
    {#if chosen.length === 0}
      Documents
    {:else if chosen.length === 1}
      {ready.find((d) => d.id === chosen[0])?.name}
    {:else}
      {chosen.length} documents
    {/if}
  </button>

  {#if open}
    <div class="menu" role="dialog" aria-label="Documents for this chat">
      {#if !modelReady}
        <p>To chat with documents, first download the document search model.</p>
        <button type="button" class="btn btn-primary small" onclick={goToDocuments}>Go to Documents</button>
      {:else if ready.length === 0}
        <p>You have no documents yet.</p>
        <button type="button" class="btn btn-primary small" onclick={goToDocuments}>Add documents</button>
      {:else}
        <p class="muted">The model will answer from the chosen documents.</p>
        <ul>
          {#each ready as d (d.id)}
            <li>
              <label>
                <input type="checkbox" checked={$chatDocs.includes(d.id)} onchange={() => toggle(d.id)} />
                <span title={d.name}>{d.name}</span>
              </label>
            </li>
          {/each}
        </ul>
        <div class="actions">
          <button type="button" class="link" onclick={goToDocuments}>Manage documents</button>
          {#if chosen.length > 0}
            <button type="button" class="link" onclick={() => chatDocs.set([])}>Clear</button>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 260px;
    padding: 3px 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-muted);
    font: inherit;
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
  }

  .toggle:hover {
    color: var(--text);
  }

  .toggle.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .menu {
    position: absolute;
    bottom: calc(100% + 6px);
    left: 0;
    z-index: 10;
    width: 320px;
    max-height: 340px;
    overflow-y: auto;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    box-shadow: var(--shadow);
    font-size: 0.88rem;
  }

  .menu p {
    margin: 0 0 8px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  label {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 2px;
    cursor: pointer;
  }

  label span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  input {
    accent-color: var(--accent);
  }

  .actions {
    display: flex;
    justify-content: space-between;
    margin-top: 8px;
  }

  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
  }

  .btn.small {
    padding: 4px 10px;
    font-size: 0.85rem;
  }
</style>
