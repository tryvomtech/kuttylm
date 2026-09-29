<script lang="ts">
  // An on/off switch. It is a real checkbox, so keyboard and screen readers work.
  let {
    checked = $bindable(),
    label,
    help,
    disabled = false,
  }: { checked: boolean; label: string; help?: string; disabled?: boolean } = $props();
</script>

<label class="switch" class:disabled>
  <input type="checkbox" role="switch" bind:checked {disabled} />
  <span class="track" aria-hidden="true"><span class="knob"></span></span>
  <span class="text">
    <span class="label">{label}</span>
    {#if help}<span class="help">{help}</span>{/if}
  </span>
</label>

<style>
  .switch {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    cursor: pointer;
  }

  .switch.disabled {
    opacity: 0.55;
    cursor: default;
  }

  input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }

  .track {
    flex-shrink: 0;
    width: 40px;
    height: 22px;
    margin-top: 1px;
    border-radius: 999px;
    background: var(--bar-bg);
    border: 1px solid var(--border);
    position: relative;
    transition: background 0.15s;
  }

  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    transition: transform 0.15s;
  }

  input:checked + .track {
    background: var(--accent);
    border-color: var(--accent);
  }

  input:checked + .track .knob {
    transform: translateX(18px);
  }

  input:focus-visible + .track {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .text {
    display: flex;
    flex-direction: column;
  }

  .label {
    font-weight: 600;
  }

  .help {
    color: var(--text-muted);
    font-size: 0.86rem;
  }
</style>
