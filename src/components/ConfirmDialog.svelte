<script lang="ts">
  // A modal "Are you sure?" box. Esc or "Keep" closes it without action.
  let {
    open,
    title,
    message,
    confirmLabel,
    cancelLabel = "Keep",
    danger = false,
    onconfirm,
    oncancel,
  }: {
    open: boolean;
    title: string;
    message: string;
    confirmLabel: string;
    cancelLabel?: string;
    danger?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();

  let dialog: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  });
</script>

<dialog bind:this={dialog} oncancel={(e) => { e.preventDefault(); oncancel(); }}>
  <h2>{title}</h2>
  <p>{message}</p>
  <div class="actions">
    <button class="btn" onclick={oncancel}>{cancelLabel}</button>
    <button class="btn" class:btn-danger={danger} class:btn-primary={!danger} onclick={onconfirm}>
      {confirmLabel}
    </button>
  </div>
</dialog>

<style>
  dialog {
    width: min(420px, calc(100vw - 32px));
    padding: 20px 22px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
  }

  dialog::backdrop {
    background: rgba(0, 0, 0, 0.45);
  }

  h2 {
    font-size: 1.1rem;
    margin-bottom: 8px;
  }

  p {
    margin: 0 0 18px;
    color: var(--text-muted);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }

  .btn-danger {
    background: var(--bad);
    border-color: var(--bad);
    color: #fff;
  }
</style>
