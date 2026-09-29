<script lang="ts">
  // The saved chats, newest first, grouped by date. Open, rename, delete, search.
  import { tick } from "svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import Icon from "./Icon.svelte";
  import { groupChats } from "../lib/chat-groups";
  import { chatList, currentChat, openChat, removeChat, renameChat } from "../lib/chat-session";
  import type { ChatSummary } from "../lib/types";

  let { saving }: { saving: boolean } = $props();

  let query = $state("");
  let editingId = $state<string | null>(null);
  let editTitle = $state("");
  let editInput: HTMLInputElement | undefined = $state();
  let toDelete = $state<ChatSummary | null>(null);
  let error = $state<string | null>(null);

  let shown = $derived(
    query.trim()
      ? $chatList.filter((c) => c.title.toLowerCase().includes(query.trim().toLowerCase()))
      : $chatList,
  );
  let groups = $derived(groupChats(shown));

  async function open(id: string) {
    error = null;
    try {
      await openChat(id);
    } catch {
      error = "This chat could not be opened. It may have been deleted.";
    }
  }

  async function startRename(chat: ChatSummary) {
    editingId = chat.id;
    editTitle = chat.title;
    await tick();
    editInput?.select();
  }

  async function finishRename() {
    const id = editingId;
    editingId = null;
    if (id && editTitle.trim()) await renameChat(id, editTitle);
  }

  function onEditKey(e: KeyboardEvent) {
    if (e.key === "Enter") void finishRename();
    if (e.key === "Escape") editingId = null;
  }

  async function confirmDelete() {
    const chat = toDelete;
    toDelete = null;
    if (chat) await removeChat(chat.id);
  }
</script>

<aside class="history" aria-label="Saved chats">
  <input type="search" placeholder="Search chats" aria-label="Search chats" bind:value={query} />

  {#if error}<p class="error">{error}</p>{/if}

  <div class="list">
    {#if $chatList.length === 0}
      <p class="empty">
        {saving ? "Your chats will appear here." : "Chat history is off. Turn it on in Settings."}
      </p>
    {:else if shown.length === 0}
      <p class="empty">No chat matches your search.</p>
    {/if}

    {#each groups as group (group.label)}
      <p class="group">{group.label}</p>
      <ul>
        {#each group.chats as chat (chat.id)}
          <li class:active={$currentChat?.id === chat.id}>
            {#if editingId === chat.id}
              <input
                class="rename"
                bind:this={editInput}
                bind:value={editTitle}
                onkeydown={onEditKey}
                onblur={finishRename}
                aria-label="New chat name"
                maxlength="120"
              />
            {:else}
              <button class="item" onclick={() => open(chat.id)} title={chat.title}>
                {chat.title}
              </button>
              <span class="actions">
                <button class="icon" onclick={() => startRename(chat)} aria-label="Rename {chat.title}" title="Rename">
                  <Icon name="edit" size={15} />
                </button>
                <button class="icon" onclick={() => (toDelete = chat)} aria-label="Delete {chat.title}" title="Delete">
                  <Icon name="trash" size={15} />
                </button>
              </span>
            {/if}
          </li>
        {/each}
      </ul>
    {/each}
  </div>
</aside>

<ConfirmDialog
  open={toDelete !== null}
  title="Delete this chat?"
  message={`"${toDelete?.title ?? ""}" will be removed from this computer.`}
  confirmLabel="Delete"
  danger
  onconfirm={confirmDelete}
  oncancel={() => (toDelete = null)}
/>

<style>
  .history {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 260px;
    flex-shrink: 0;
    padding: 14px 10px;
    border-right: 1px solid var(--border);
    background: var(--surface);
    min-height: 0;
  }

  input[type="search"],
  .rename {
    width: 100%;
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 0.9rem;
  }

  input:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .list {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .group {
    margin: 12px 8px 4px;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: 8px;
  }

  li:hover,
  li.active {
    background: var(--surface-2);
  }

  li.active .item {
    font-weight: 600;
    color: var(--text);
  }

  .item {
    flex: 1;
    min-width: 0;
    padding: 7px 8px;
    border: none;
    background: none;
    text-align: left;
    color: var(--text-muted);
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .actions {
    display: none;
    gap: 2px;
    padding-right: 4px;
  }

  li:hover .actions,
  li.active .actions,
  li:focus-within .actions {
    display: flex;
  }

  .icon {
    display: flex;
    padding: 4px;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--text-muted);
  }

  .icon:hover {
    background: var(--border);
    color: var(--text);
  }

  .empty {
    margin: 12px 8px;
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .error {
    margin: 0 8px;
    font-size: 0.82rem;
    color: var(--bad);
  }
</style>
