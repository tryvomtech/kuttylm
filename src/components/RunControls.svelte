<script lang="ts">
  // Run / Stop for one downloaded model, with loading and error states.
  import { errorText, startModel, stopModel } from "../lib/api";
  import { engine } from "../lib/engine";

  let { modelId }: { modelId: string } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  let mine = $derived($engine.model_id === modelId);
  let runState = $derived(mine ? $engine.state : "stopped");
  let otherRunning = $derived(!mine && ($engine.state === "running" || $engine.state === "starting"));

  async function act(action: () => Promise<void>) {
    busy = true;
    error = null;
    try {
      await action();
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="run">
  {#if runState === "starting"}
    <p class="status"><span class="spinner" aria-hidden="true"></span> Loading the model…</p>
    <button class="btn" disabled={busy} onclick={() => act(stopModel)}>Stop</button>
  {:else if runState === "running"}
    <p class="status running">
      <span class="dot" aria-hidden="true"></span>
      Running · {$engine.uses_gpu ? "graphics card" : "processor"}
    </p>
    {#if $engine.note}<p class="note">{$engine.note}</p>{/if}
    {#if $engine.needs_restart}
      <p class="note">Settings changed. Restart to use them.</p>
    {/if}
    <div class="buttons">
      {#if $engine.needs_restart}
        <button class="btn" disabled={busy} onclick={() => act(() => startModel(modelId))}>Restart</button>
      {/if}
      <button class="btn" disabled={busy} onclick={() => act(stopModel)}>Stop</button>
    </div>
  {:else}
    {#if runState === "failed" && $engine.error}
      <p class="error" role="alert">{$engine.error}</p>
      {#if $engine.log_tail.length > 0}
        <details>
          <summary>Show details</summary>
          <pre>{$engine.log_tail.join("\n")}</pre>
        </details>
      {/if}
    {/if}
    <button
      class="btn btn-primary"
      disabled={busy}
      title={otherRunning ? `${$engine.model_name ?? "The other model"} will stop first` : undefined}
      onclick={() => act(() => startModel(modelId))}
    >
      {runState === "failed" ? "Try again" : "Run"}
    </button>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</div>

<style>
  .run {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    text-align: right;
    max-width: 100%;
  }

  p {
    margin: 0;
    font-size: 0.85rem;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
  }

  .status.running {
    color: var(--good);
    font-weight: 600;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
  }

  .spinner {
    width: 12px;
    height: 12px;
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

  .note {
    color: var(--warn);
  }

  .buttons {
    display: flex;
    gap: 8px;
  }

  .error {
    color: var(--bad);
  }

  details {
    width: 100%;
    text-align: left;
    font-size: 0.8rem;
  }

  summary {
    cursor: pointer;
    color: var(--text-muted);
    text-align: right;
  }

  pre {
    margin: 6px 0 0;
    padding: 8px;
    max-height: 180px;
    overflow: auto;
    background: var(--surface-2);
    border-radius: 6px;
    white-space: pre-wrap;
    word-break: break-all;
    font-size: 0.72rem;
  }
</style>
