<script lang="ts" module>
  export interface Part {
    code: boolean;
    text: string;
  }

  /** Splits text at ``` fences. An open fence (still arriving) counts as code. */
  export function splitCode(text: string): Part[] {
    const parts: Part[] = [];
    const pieces = text.split("```");
    pieces.forEach((piece, i) => {
      const code = i % 2 === 1;
      // Drop the language name after the opening fence, e.g. ```python
      const body = code ? piece.replace(/^[\w+-]*\n/, "") : piece;
      if (body.trim()) parts.push({ code, text: code ? body.replace(/\n$/, "") : body });
    });
    return parts;
  }
</script>

<script lang="ts">
  // Shows model text safely: plain text, with code blocks in a box. Never as HTML.
  let { text }: { text: string } = $props();
  let parts = $derived(splitCode(text));
</script>

{#each parts as part, i (i)}
  {#if part.code}
    <pre class="code">{part.text}</pre>
  {:else}
    <p class="text">{part.text.trim()}</p>
  {/if}
{/each}

<style>
  .text {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .text + .text,
  .code + .text,
  .text + .code {
    margin-top: 8px;
  }

  .code {
    margin: 8px 0;
    padding: 10px 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-family: ui-monospace, "Ubuntu Mono", monospace;
    font-size: 0.86rem;
    overflow-x: auto;
    white-space: pre;
  }
</style>
