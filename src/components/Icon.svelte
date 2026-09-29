<script lang="ts" module>
  // Small line icons (24×24, stroke = currentColor). No icon library needed.
  export type IconName =
    | "home"
    | "browse"
    | "models"
    | "chat"
    | "settings"
    | "sun"
    | "moon"
    | "monitor"
    | "refresh"
    | "memory"
    | "cpu"
    | "gpu";
</script>

<script lang="ts">
  let { name, size = 20 }: { name: IconName; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.8"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "home"}
    <path d="M3 10.5 12 3l9 7.5" /><path d="M5 9.5V20h14V9.5" />
  {:else if name === "browse"}
    <circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" />
  {:else if name === "models"}
    <path d="M12 3 3 7.5l9 4.5 9-4.5L12 3Z" /><path d="m3 12 9 4.5 9-4.5" /><path d="m3 16.5 9 4.5 9-4.5" />
  {:else if name === "chat"}
    <path d="M4 5h16v11H9l-5 4V5Z" />
  {:else if name === "settings"}
    <circle cx="12" cy="12" r="3" />
    <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1" />
  {:else if name === "sun"}
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
  {:else if name === "moon"}
    <path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5Z" />
  {:else if name === "monitor"}
    <rect x="3" y="4" width="18" height="12" rx="2" /><path d="M8 20h8M12 16v4" />
  {:else if name === "refresh"}
    <path d="M20 11a8 8 0 0 0-14.3-4.9L4 8" /><path d="M4 3v5h5" />
    <path d="M4 13a8 8 0 0 0 14.3 4.9L20 16" /><path d="M20 21v-5h-5" />
  {:else if name === "memory"}
    <rect x="3" y="7" width="18" height="10" rx="1.5" /><path d="M7 7v10M11 7v10M15 7v10M3 20h18" />
  {:else if name === "cpu"}
    <rect x="6" y="6" width="12" height="12" rx="1.5" /><rect x="9.5" y="9.5" width="5" height="5" />
    <path d="M9 3v3M15 3v3M9 18v3M15 18v3M3 9h3M3 15h3M18 9h3M18 15h3" />
  {:else if name === "gpu"}
    <rect x="2" y="6" width="20" height="11" rx="1.5" /><circle cx="8" cy="11.5" r="2.5" />
    <circle cx="16" cy="11.5" r="2.5" /><path d="M5 17v3" />
  {/if}
</svg>
