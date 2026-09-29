<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Icon, { type IconName } from "./Icon.svelte";
  import { getAppVersion } from "../lib/api";
  import { engine } from "../lib/engine";
  import { currentScreen, theme, type Screen, type Theme } from "../lib/stores";

  const RELEASES = "https://github.com/tryvomtech/kuttylm/releases";
  let version = $state("");

  onMount(() => {
    getAppVersion()
      .then((v) => (version = v))
      .catch(() => {});
  });

  const items: { id: Screen; label: string; icon: IconName }[] = [
    { id: "home", label: "My computer", icon: "home" },
    { id: "browse", label: "Find models", icon: "browse" },
    { id: "models", label: "My models", icon: "models" },
    { id: "chat", label: "Chat", icon: "chat" },
    { id: "settings", label: "Settings", icon: "settings" },
  ];

  const nextTheme: Record<Theme, Theme> = { system: "light", light: "dark", dark: "system" };
  const themeLabel: Record<Theme, string> = {
    system: "Theme: system",
    light: "Theme: light",
    dark: "Theme: dark",
  };
  const themeIcon: Record<Theme, IconName> = { system: "monitor", light: "sun", dark: "moon" };
</script>

<nav class="sidebar">
  <div class="brand">
    <img src="/icon.png" alt="" width="28" height="28" />
    <span>KuttyLM</span>
  </div>

  <ul>
    {#each items as item (item.id)}
      <li>
        <button
          class="nav-item"
          class:active={$currentScreen === item.id}
          aria-current={$currentScreen === item.id ? "page" : undefined}
          onclick={() => currentScreen.set(item.id)}
        >
          <Icon name={item.icon} />
          <span>{item.label}</span>
        </button>
      </li>
    {/each}
  </ul>

  <button class="engine" onclick={() => currentScreen.set("models")} title="Open My models">
    {#if $engine.state === "running"}
      <span class="dot on" aria-hidden="true"></span>
      <span><strong>{$engine.model_name}</strong><br />is running</span>
    {:else if $engine.state === "starting"}
      <span class="dot wait" aria-hidden="true"></span>
      <span>Loading {$engine.model_name}…</span>
    {:else if $engine.state === "failed"}
      <span class="dot bad" aria-hidden="true"></span>
      <span>The model could not start</span>
    {:else}
      <span class="dot" aria-hidden="true"></span>
      <span>No model running</span>
    {/if}
  </button>

  <button class="nav-item theme" onclick={() => theme.set(nextTheme[$theme])} title="Change theme">
    <Icon name={themeIcon[$theme]} />
    <span>{themeLabel[$theme]}</span>
  </button>

  {#if version}
    <button
      class="version"
      onclick={() => void openUrl(`${RELEASES}/tag/v${version}`).catch(() => {})}
      title="See what is new in this version"
    >
      KuttyLM {version}
    </button>
  {/if}
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 220px;
    flex-shrink: 0;
    padding: 16px 12px;
    background: var(--surface);
    border-right: 1px solid var(--border);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 10px 18px;
    font-weight: 700;
    font-size: 1.15rem;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px 10px;
    border: none;
    border-radius: 8px;
    background: none;
    color: var(--text-muted);
    text-align: left;
  }

  .nav-item:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .nav-item.active {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .version {
    align-self: flex-start;
    margin: 2px 0 0 10px;
    padding: 2px 0;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 0.75rem;
    opacity: 0.8;
  }

  .version:hover {
    opacity: 1;
    text-decoration: underline;
  }

  .theme {
    font-size: 0.85rem;
  }

  .engine {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin: 0 0 8px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: 0.82rem;
    text-align: left;
    line-height: 1.35;
  }

  .engine strong {
    color: var(--text);
    overflow-wrap: anywhere;
  }

  .dot {
    flex-shrink: 0;
    width: 8px;
    height: 8px;
    margin-top: 5px;
    border-radius: 50%;
    background: var(--border);
  }

  .dot.on {
    background: var(--good);
  }

  .dot.wait {
    background: var(--warn);
  }

  .dot.bad {
    background: var(--bad);
  }
</style>
