<script lang="ts">
  // Shows a value in a box with a Copy button.
  let { value, label }: { value: string; label: string } = $props();

  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
    } catch {
      // Older WebKit: copy through a hidden text field.
      const area = document.createElement("textarea");
      area.value = value;
      document.body.appendChild(area);
      area.select();
      document.execCommand("copy");
      area.remove();
    }
    copied = true;
    clearTimeout(timer);
    timer = setTimeout(() => (copied = false), 1500);
  }
</script>

<div class="copy">
  <span class="label">{label}</span>
  <div class="row">
    <code>{value}</code>
    <button type="button" class="btn" onclick={copy} aria-label="Copy {label}">
      {copied ? "Copied" : "Copy"}
    </button>
  </div>
</div>

<style>
  .copy {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label {
    font-size: 0.86rem;
    color: var(--text-muted);
  }

  .row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  code {
    flex: 1;
    min-width: 0;
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    font-family: ui-monospace, "Ubuntu Mono", monospace;
    font-size: 0.9rem;
    overflow-wrap: anywhere;
    user-select: all;
  }

  .btn {
    min-width: 76px;
    justify-content: center;
  }
</style>
