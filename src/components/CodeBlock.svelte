<script lang="ts">
  // A code block with its language and a Copy button.
  let { text, lang = "" }: { text: string; lang?: string } = $props();

  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      const area = document.createElement("textarea");
      area.value = text;
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

<div class="code">
  <div class="bar">
    <span>{lang || "code"}</span>
    <button type="button" onclick={copy} aria-label="Copy code">{copied ? "Copied" : "Copy"}</button>
  </div>
  <pre><code>{text}</code></pre>
</div>

<style>
  .code {
    margin: 10px 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    background: var(--surface-2);
  }

  .bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 8px 4px 12px;
    border-bottom: 1px solid var(--border);
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  button {
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    font-size: 0.75rem;
  }

  pre {
    margin: 0;
    padding: 10px 12px;
    overflow-x: auto;
    font-family: ui-monospace, "Ubuntu Mono", monospace;
    font-size: 0.86rem;
    line-height: 1.5;
  }
</style>
