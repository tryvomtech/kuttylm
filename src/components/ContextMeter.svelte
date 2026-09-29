<script lang="ts">
  // "Chat memory": how much of the model's context window the open chat uses.
  import { context } from "../lib/chat-session";

  let pct = $derived(
    $context.size && $context.used != null ? Math.min(100, Math.round(($context.used / $context.size) * 100)) : null,
  );
  let level = $derived(pct == null ? "ok" : pct >= 90 ? "full" : pct >= 75 ? "high" : "ok");
  const fmt = (n: number) => n.toLocaleString();
</script>

{#if $context.size}
  <div
    class="meter {level}"
    title="How much of the model's memory (context window) this chat uses. When it is full, the model reads only the newest messages."
  >
    <div class="top">
      <span class="label">
        {#if $context.loading}
          <span class="spinner" aria-hidden="true"></span> Loading this chat into the model…
        {:else}
          Chat memory
        {/if}
      </span>
      {#if pct != null}
        <span class="numbers">{pct}% · {fmt($context.used ?? 0)} of {fmt($context.size)} tokens</span>
      {/if}
    </div>
    <div
      class="bar"
      role="progressbar"
      aria-label="Chat memory used"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={pct ?? 0}
    >
      <div class="fill" style:width="{pct ?? 0}%"></div>
    </div>
    {#if level === "full" || $context.dropped > 0}
      <p class="hint">The model's memory is almost full. Start a new chat for a new topic.</p>
    {:else if level === "high"}
      <p class="hint">The chat is getting long. Soon the model will forget the oldest messages.</p>
    {/if}
  </div>
{/if}

<style>
  .meter {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .top {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }

  .label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .numbers {
    font-variant-numeric: tabular-nums;
  }

  .bar {
    height: 4px;
    border-radius: 999px;
    background: var(--bar-bg);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease;
  }

  .high .fill {
    background: var(--warn);
  }

  .full .fill {
    background: var(--bad);
  }

  .high .hint {
    color: var(--warn);
  }

  .full .hint {
    color: var(--bad);
  }

  .hint {
    margin: 0;
  }

  .spinner {
    width: 10px;
    height: 10px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
