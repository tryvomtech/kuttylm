<script lang="ts" module>
  import type { FitLabel } from "../lib/types";

  // Words shown to the user. The rules behind them live in src-tauri/src/fit.rs.
  export const FIT_TEXT: Record<FitLabel, { label: string; help: string }> = {
    fits_well: {
      label: "Fits well",
      help: "This model fits in your computer's free memory.",
    },
    may_be_slow: {
      label: "May be slow",
      help: "Too big for your graphics card. It will run on the processor, which is slower.",
    },
    will_not_fit: {
      label: "Will not fit",
      help: "Your computer does not have enough free memory. Closing other programs may help.",
    },
  };
</script>

<script lang="ts">
  let { fit }: { fit: FitLabel } = $props();
</script>

<span class="badge {fit}" title={FIT_TEXT[fit].help}>
  <span class="dot" aria-hidden="true"></span>
  {FIT_TEXT[fit].label}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px;
    border-radius: 999px;
    border: 1px solid currentColor;
    font-size: 0.82rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
  }

  .fits_well {
    color: var(--good);
  }

  .may_be_slow {
    color: var(--warn);
  }

  .will_not_fit {
    color: var(--bad);
  }
</style>
