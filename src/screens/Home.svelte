<script lang="ts">
  import { onMount } from "svelte";
  import Card from "../components/Card.svelte";
  import Icon from "../components/Icon.svelte";
  import MeterBar from "../components/MeterBar.svelte";
  import Notice from "../components/Notice.svelte";
  import { errorText, getDeviceInfo } from "../lib/api";
  import { formatBytes, percent } from "../lib/format";
  import type { DeviceInfo, GpuInfo } from "../lib/types";

  let info = $state<DeviceInfo | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);

  async function load() {
    loading = true;
    error = null;
    try {
      info = await getDeviceInfo();
    } catch (err) {
      error = errorText(err);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  function gpuMemoryText(gpu: GpuInfo): string {
    if (gpu.integrated) return "Uses the computer's main memory";
    if (gpu.vram_total_bytes == null) return "Unknown";
    const total = formatBytes(gpu.vram_total_bytes);
    if (gpu.vram_free_bytes == null) return total;
    return `${total} (${formatBytes(gpu.vram_free_bytes)} free)`;
  }

  let ramUsed = $derived(info ? info.ram_total_bytes - info.ram_available_bytes : 0);
</script>

<div class="page">
  <header>
    <div>
      <h1>My computer</h1>
      <p class="muted">
        {info ? info.os_name : "Checking your computer…"}
      </p>
    </div>
    <button class="btn" onclick={load} disabled={loading}>
      <Icon name="refresh" size={16} />
      {loading ? "Checking…" : "Check again"}
    </button>
  </header>

  {#if error}
    <Notice kind="error">
      <strong>Could not read your computer's details.</strong>
      <span>{error}</span>
    </Notice>
  {/if}

  {#if info}
    <div class="grid">
      <Card title="Memory (RAM)" icon="memory">
        <p class="big">{formatBytes(info.ram_total_bytes)}</p>
        <MeterBar value={percent(ramUsed, info.ram_total_bytes)} label="Memory in use" />
        <p class="muted small">
          {formatBytes(info.ram_available_bytes)} free now · {formatBytes(ramUsed)} in use
        </p>
      </Card>

      <Card title="Processor (CPU)" icon="cpu">
        <p class="name">{info.cpu_name}</p>
        <dl>
          {#if info.cpu_physical_cores != null}
            <dt>Cores</dt>
            <dd>{info.cpu_physical_cores}</dd>
          {/if}
          <dt>Threads</dt>
          <dd>{info.cpu_threads}</dd>
        </dl>
      </Card>

      <div class="wide">
        <Card title={info.gpus.length > 1 ? "Graphics cards (GPU)" : "Graphics card (GPU)"} icon="gpu">
          {#if info.gpus.length === 0}
            <p class="name">Not found</p>
            <p class="muted small">Models will run on the processor. This works, but it is slower.</p>
          {:else}
            <div class="gpus">
              {#each info.gpus as gpu, i (i)}
                <div class="gpu">
                  <p class="name">{gpu.name}</p>
                  <dl>
                    <dt>Type</dt>
                    <dd>{gpu.integrated ? "Built into the processor" : "Separate card"}</dd>
                    <dt>Graphics memory</dt>
                    <dd>{gpuMemoryText(gpu)}</dd>
                  </dl>
                </div>
              {/each}
            </div>
          {/if}
        </Card>
      </div>
    </div>

    <p class="muted small hint">
      KuttyLM uses these details to tell you which models will work well on this computer.
    </p>
  {:else if loading}
    <p class="muted">Checking your computer…</p>
  {/if}
</div>

<style>
  .page {
    padding: 28px 32px;
    max-width: 1100px;
  }

  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 22px;
  }

  h1 {
    font-size: 1.5rem;
    margin-bottom: 4px;
  }

  p {
    margin: 0;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }

  .wide {
    grid-column: 1 / -1;
  }

  @media (max-width: 900px) {
    .grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .big {
    font-size: 1.8rem;
    font-weight: 700;
    margin-bottom: 10px;
  }

  .name {
    font-weight: 600;
    font-size: 1.05rem;
    margin-bottom: 8px;
  }

  .small {
    font-size: 0.85rem;
    margin-top: 8px;
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 14px;
    margin: 0;
  }

  dt {
    color: var(--text-muted);
  }

  dd {
    margin: 0;
  }

  .gpus {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 14px 28px;
  }

  .hint {
    margin-top: 18px;
  }
</style>
