<script lang="ts">
  import { onMount, tick } from "svelte";
  import ChatHistory from "../components/ChatHistory.svelte";
  import ContextMeter from "../components/ContextMeter.svelte";
  import Icon from "../components/Icon.svelte";
  import Markdown from "../components/Markdown.svelte";
  import MessageText from "../components/MessageText.svelte";
  import { getSettings } from "../lib/api";
  import { busy, currentChat, entries, newChat, refreshChatList, retry, send, stop } from "../lib/chat-session";
  import { engine } from "../lib/engine";
  import { currentScreen } from "../lib/stores";

  const PANEL_KEY = "kuttylm.chatHistoryOpen";
  let showHistory = $state(readPanel());
  let saving = $state(true);

  function readPanel(): boolean {
    try {
      return localStorage.getItem(PANEL_KEY) !== "false";
    } catch {
      return true;
    }
  }

  function togglePanel() {
    showHistory = !showHistory;
    try {
      localStorage.setItem(PANEL_KEY, String(showHistory));
    } catch {
      // Only a comfort setting.
    }
  }

  onMount(() => {
    void refreshChatList();
    getSettings()
      .then((s) => (saving = s.save_chats))
      .catch(() => {});
  });

  let draft = $state("");
  let list: HTMLElement | undefined = $state();
  let input: HTMLTextAreaElement | undefined = $state();
  /** Follow new text only while the user is at the bottom. */
  let follow = true;

  let running = $derived($engine.state === "running");

  function onScroll() {
    if (!list) return;
    follow = list.scrollHeight - list.scrollTop - list.clientHeight < 60;
  }

  // Scroll down when new text arrives.
  $effect(() => {
    void $entries;
    if (!follow || !list) return;
    void tick().then(() => list?.scrollTo({ top: list.scrollHeight }));
  });

  // Grow the input box with its text (up to a limit set in CSS).
  $effect(() => {
    void draft;
    if (!input) return;
    input.style.height = "auto";
    // + the top and bottom border (the box uses border-box sizing), so no scroll bar appears.
    input.style.height = `${input.scrollHeight + input.offsetHeight - input.clientHeight}px`;
  });

  function submit() {
    if (!draft.trim() || $busy || !running) return;
    follow = true;
    void send(draft);
    draft = "";
  }

  function onKeydown(e: KeyboardEvent) {
    // Enter sends; Shift+Enter makes a new line.
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      submit();
    }
  }

  async function startNew() {
    await newChat();
    draft = "";
    input?.focus();
  }
</script>

<div class="screen">
{#if showHistory}<ChatHistory {saving} />{/if}
<div class="chat">
  <header>
    <button class="btn icon-btn" onclick={togglePanel} aria-pressed={showHistory} title={showHistory ? "Hide chat history" : "Show chat history"}>
      <Icon name="panel" size={18} />
    </button>
    <div class="head-text">
      <h1 title={$currentChat?.title}>{$currentChat?.title ?? "New chat"}</h1>
      <p class="muted">
        {#if running}
          Talking to <strong>{$engine.model_name}</strong>
        {:else if $engine.state === "starting"}
          Loading {$engine.model_name}…
        {:else}
          No model is running
        {/if}
      </p>
    </div>
    <button class="btn" onclick={startNew} disabled={$entries.length === 0 && !$currentChat}>
      <Icon name="plus" size={16} /> New chat
    </button>
  </header>

  <div class="messages" bind:this={list} onscroll={onScroll}>
    {#if $entries.length === 0}
      <div class="empty">
        {#if running}
          <p>Ask anything. The model runs on this computer, so your chat is not sent to the internet.</p>
        {:else if $engine.state === "starting"}
          <p>The model is loading. You can type when it is ready.</p>
        {:else}
          <p>Start a model first.</p>
          <button class="btn btn-primary" onclick={() => currentScreen.set("models")}>Go to My models</button>
        {/if}
      </div>
    {/if}

    {#each $entries as e (e.id)}
      {#if e.role === "notice"}
        <p class="notice">{e.content}</p>
      {:else if e.role === "user"}
        <div class="row user">
          <div class="bubble"><MessageText text={e.content} /></div>
        </div>
      {:else}
        <div class="row assistant">
          <div class="bubble">
            {#if e.reasoning}
              <details class="thinking">
                <summary>{e.pending && !e.content ? "Thinking…" : "Show thinking"}</summary>
                <p>{e.reasoning}</p>
              </details>
            {/if}
            {#if e.content}
              <Markdown text={e.content} />
            {:else if e.pending && !e.reasoning}
              <span class="typing" aria-label="The model is writing"><i></i><i></i><i></i></span>
            {/if}
            {#if e.error}
              <p class="error" role="alert">{e.error}</p>
              <button class="btn small" onclick={retry} disabled={$busy || !running}>Try again</button>
            {/if}
            {#if !e.pending && !e.error}
              <p class="meta">
                {e.modelName ?? ""}
                {#if e.tokensPerSecond}· about {Math.round(e.tokensPerSecond * 0.75)} words per second{/if}
                {#if e.stopped}· stopped{/if}
                {#if e.cut}· cut at the length limit{/if}
              </p>
            {/if}
          </div>
        </div>
      {/if}
    {/each}
  </div>

  <div class="footer">
  <div class="memory"><ContextMeter /></div>

  <form
    class="composer"
    onsubmit={(e) => {
      e.preventDefault();
      submit();
    }}
  >
    <textarea
      bind:this={input}
      bind:value={draft}
      onkeydown={onKeydown}
      rows="1"
      placeholder={running ? "Type a message. Enter sends, Shift+Enter for a new line." : "Start a model to chat."}
      disabled={!running}
      aria-label="Message"
    ></textarea>
    {#if $busy}
      <button type="button" class="btn" onclick={stop}>Stop</button>
    {:else}
      <button type="submit" class="btn btn-primary" disabled={!running || !draft.trim()}>Send</button>
    {/if}
  </form>
  </div>
</div>
</div>

<style>
  .screen {
    display: flex;
    height: 100%;
    min-height: 0;
  }

  .chat {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    height: 100%;
  }

  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 18px 32px 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .head-text {
    flex: 1;
    min-width: 0;
  }

  .icon-btn {
    padding: 7px;
  }

  h1 {
    font-size: 1.3rem;
    margin: 0 0 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  header p {
    margin: 0;
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 20px 32px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .empty {
    margin: auto;
    max-width: 420px;
    text-align: center;
    color: var(--text-muted);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  .empty p {
    margin: 0;
  }

  .row {
    display: flex;
  }

  .row.user {
    justify-content: flex-end;
  }

  .bubble {
    max-width: min(760px, 85%);
    padding: 10px 14px;
    border-radius: 14px;
    border: 1px solid var(--border);
    background: var(--surface);
    min-width: 0;
  }

  .user .bubble {
    background: var(--accent-soft);
    border-color: transparent;
    border-bottom-right-radius: 4px;
  }

  .assistant .bubble {
    border-bottom-left-radius: 4px;
  }

  .meta {
    margin: 6px 0 0;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .notice {
    align-self: center;
    margin: 0;
    padding: 2px 12px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: 0.8rem;
  }

  .error {
    margin: 4px 0 8px;
    color: var(--bad);
  }

  .btn.small {
    padding: 4px 10px;
    font-size: 0.85rem;
  }

  .thinking {
    margin-bottom: 6px;
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .thinking summary {
    cursor: pointer;
  }

  .thinking p {
    margin: 6px 0 0;
    white-space: pre-wrap;
  }

  .typing {
    display: inline-flex;
    gap: 4px;
    padding: 6px 0;
  }

  .typing i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-muted);
    animation: blink 1.2s infinite ease-in-out;
  }

  .typing i:nth-child(2) {
    animation-delay: 0.2s;
  }

  .typing i:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes blink {
    0%,
    80%,
    100% {
      opacity: 0.25;
    }
    40% {
      opacity: 1;
    }
  }

  .footer {
    border-top: 1px solid var(--border);
    background: var(--bg);
  }

  .memory {
    padding: 8px 32px 0;
  }

  .memory:empty {
    display: none;
  }

  .composer {
    display: flex;
    align-items: flex-end;
    gap: 10px;
    padding: 10px 32px 20px;
  }

  textarea {
    flex: 1;
    resize: none;
    max-height: 200px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    line-height: 1.4;
  }

  textarea:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  textarea:disabled {
    opacity: 0.6;
  }
</style>
