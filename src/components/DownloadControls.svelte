<script lang="ts">
  // Download button, progress, Pause / Resume / Cancel for one model.
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import ProgressBar from "./ProgressBar.svelte";
  import { cancelDownload, errorText, pauseDownload, startDownload } from "../lib/api";
  import { downloads, isActive } from "../lib/downloads";
  import { formatBytes, formatDuration, formatSpeed, percent } from "../lib/format";
  import type { LocalStatus } from "../lib/types";

  let {
    modelId,
    totalBytes,
    local,
    bytesOnDisk = 0,
  }: {
    modelId: string;
    totalBytes: number;
    /** What is on disk now; `null` if nothing. */
    local: LocalStatus | null;
    bytesOnDisk?: number;
  } = $props();

  let error = $state<string | null>(null);
  let busy = $state(false);
  let confirmCancel = $state(false);

  let event = $derived($downloads[modelId]);
  let active = $derived(isActive(event));
  let stopped = $derived(!active && (local === "incomplete" || event?.status === "paused" || event?.status === "failed"));
  let done = $derived(event?.downloaded_bytes ?? bytesOnDisk);
  let total = $derived(event?.total_bytes ?? totalBytes);
  let shownError = $derived(error ?? (event?.status === "failed" ? event.error : null));

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

  function doCancel() {
    confirmCancel = false;
    void act(() => cancelDownload(modelId));
  }
</script>

<div class="controls">
  {#if local === "ready" && !event}
    <span class="ready">✓ Downloaded</span>
  {:else if active || stopped}
    <div class="progress">
      <ProgressBar value={percent(done, total)} label="Download progress" muted={stopped} />
      <p class="line muted">
        {#if event?.status === "verifying"}
          Checking the file…
        {:else if active}
          <span>{Math.floor(percent(done, total))}% · {formatBytes(done)} of {formatBytes(total)}</span>
          {#if event && event.speed_bps > 0}
            <span>· {formatSpeed(event.speed_bps)}</span>
            {#if event.eta_secs != null}<span>· {formatDuration(event.eta_secs)} left</span>{/if}
          {/if}
        {:else}
          Paused · {formatBytes(done)} of {formatBytes(total)}
        {/if}
      </p>
    </div>
    <div class="buttons">
      {#if active}
        <button class="btn" disabled={busy || event?.status === "verifying"} onclick={() => act(() => pauseDownload(modelId))}>
          Pause
        </button>
      {:else}
        <button class="btn btn-primary" disabled={busy} onclick={() => act(() => startDownload(modelId))}>
          Resume
        </button>
      {/if}
      <button class="btn" disabled={busy} onclick={() => (confirmCancel = true)}>Cancel</button>
    </div>
  {:else}
    <button class="btn btn-primary" disabled={busy} onclick={() => act(() => startDownload(modelId))}>
      Download
    </button>
  {/if}

  {#if shownError}
    <p class="error" role="alert">{shownError}</p>
  {/if}
</div>

<ConfirmDialog
  open={confirmCancel}
  title="Cancel this download?"
  message="The part that is already downloaded will be removed."
  confirmLabel="Cancel download"
  danger
  onconfirm={doCancel}
  oncancel={() => (confirmCancel = false)}
/>

<style>
  .controls {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    width: 100%;
  }

  .controls > .btn {
    align-self: flex-end;
  }

  .progress {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .line {
    margin: 0;
    font-size: 0.8rem;
  }

  .line span {
    white-space: nowrap;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .ready {
    align-self: flex-end;
    color: var(--good);
    font-weight: 600;
  }

  .error {
    margin: 0;
    color: var(--bad);
    font-size: 0.82rem;
  }
</style>
