<script lang="ts">
  // Inline Markdown parts. Text is always shown as text, never as HTML.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import MdInline from "./MdInline.svelte";
  import type { Inline } from "../lib/markdown";

  let { parts }: { parts: Inline[] } = $props();

  function open(e: MouseEvent, href: string) {
    // Open in the normal web browser, not inside the KuttyLM window.
    e.preventDefault();
    void openUrl(href).catch(() => {});
  }
</script>

{#each parts as p, i (i)}
  {#if p.type === "text"}{p.text}{:else if p.type === "br"}<br />{:else if p.type === "code"}<code class="inline">{p.text}</code
    >{:else if p.type === "strong"}<strong><MdInline parts={p.children} /></strong>{:else if p.type === "em"}<em
      ><MdInline parts={p.children} /></em
    >{:else if p.type === "del"}<del><MdInline parts={p.children} /></del>{:else if p.type === "link"}<a
      href={p.href}
      title={p.href}
      rel="noreferrer noopener"
      onclick={(e) => open(e, p.href)}><MdInline parts={p.children} /></a
    >{/if}
{/each}

<style>
  .inline {
    padding: 1px 5px;
    border-radius: 5px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-family: ui-monospace, "Ubuntu Mono", monospace;
    font-size: 0.88em;
    overflow-wrap: anywhere;
  }

  a {
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 2px;
    overflow-wrap: anywhere;
  }
</style>
