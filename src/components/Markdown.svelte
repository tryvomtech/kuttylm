<script lang="ts">
  // Shows Markdown from a chat answer (see lib/markdown.ts). Safe: no HTML from the model.
  import CodeBlock from "./CodeBlock.svelte";
  import Markdown from "./Markdown.svelte";
  import MdInline from "./MdInline.svelte";
  import { parseMarkdown, type Block } from "../lib/markdown";

  let { text = "", blocks: given }: { text?: string; blocks?: Block[] } = $props();
  let blocks = $derived(given ?? parseMarkdown(text));
</script>

<div class="md">
  {#each blocks as b, i (i)}
    {#if b.type === "paragraph"}
      <p><MdInline parts={b.children} /></p>
    {:else if b.type === "heading"}
      <p class="h h{Math.min(b.level, 4)}" role="heading" aria-level={b.level}><MdInline parts={b.children} /></p>
    {:else if b.type === "code"}
      <CodeBlock text={b.text} lang={b.lang} />
    {:else if b.type === "list"}
      {#if b.ordered}
        <ol start={b.start}>
          {#each b.items as item, j (j)}<li><Markdown blocks={item} /></li>{/each}
        </ol>
      {:else}
        <ul>
          {#each b.items as item, j (j)}<li><Markdown blocks={item} /></li>{/each}
        </ul>
      {/if}
    {:else if b.type === "quote"}
      <blockquote><Markdown blocks={b.children} /></blockquote>
    {:else if b.type === "table"}
      <div class="table">
        <table>
          <thead>
            <tr>
              {#each b.header as cell, c (c)}<th style:text-align={b.align[c]}><MdInline parts={cell} /></th>{/each}
            </tr>
          </thead>
          <tbody>
            {#each b.rows as row, r (r)}
              <tr>
                {#each row as cell, c (c)}<td style:text-align={b.align[c]}><MdInline parts={cell} /></td>{/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if b.type === "hr"}
      <hr />
    {/if}
  {/each}
</div>

<style>
  .md {
    overflow-wrap: anywhere;
  }

  .md > :first-child {
    margin-top: 0;
  }

  .md > :last-child {
    margin-bottom: 0;
  }

  p {
    margin: 0 0 10px;
  }

  .h {
    margin: 14px 0 6px;
    font-weight: 700;
    line-height: 1.3;
  }

  .h1 {
    font-size: 1.3rem;
  }

  .h2 {
    font-size: 1.15rem;
  }

  .h3,
  .h4 {
    font-size: 1rem;
  }

  ul,
  ol {
    margin: 0 0 10px;
    padding-left: 22px;
  }

  li {
    margin: 2px 0;
  }

  /* A list item holds a small Markdown block; keep it tight. */
  li :global(.md p) {
    margin: 0;
  }

  li :global(.md ul),
  li :global(.md ol) {
    margin: 2px 0 0;
  }

  blockquote {
    margin: 0 0 10px;
    padding: 2px 12px;
    border-left: 3px solid var(--border);
    color: var(--text-muted);
  }

  hr {
    border: none;
    border-top: 1px solid var(--border);
    margin: 12px 0;
  }

  .table {
    margin: 0 0 10px;
    overflow-x: auto;
  }

  table {
    border-collapse: collapse;
    font-size: 0.92rem;
  }

  th,
  td {
    padding: 6px 10px;
    border: 1px solid var(--border);
  }

  th {
    background: var(--surface-2);
    font-weight: 600;
  }
</style>
