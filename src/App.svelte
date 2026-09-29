<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Browse from "./screens/Browse.svelte";
  import Chat from "./screens/Chat.svelte";
  import Home from "./screens/Home.svelte";
  import MyModels from "./screens/MyModels.svelte";
  import Settings from "./screens/Settings.svelte";
  import { startDownloadEvents } from "./lib/downloads";
  import { startEngineEvents } from "./lib/engine";
  import { refreshChatList } from "./lib/chat-session";
  import { currentScreen } from "./lib/stores";

  onMount(() => {
    startDownloadEvents();
    startEngineEvents();
    void refreshChatList();
  });
</script>

<div class="layout">
  <Sidebar />
  <main>
    {#if $currentScreen === "home"}
      <Home />
    {:else if $currentScreen === "browse"}
      <Browse />
    {:else if $currentScreen === "models"}
      <MyModels />
    {:else if $currentScreen === "chat"}
      <Chat />
    {:else}
      <Settings />
    {/if}
  </main>
</div>

<style>
  .layout {
    display: flex;
    height: 100%;
  }

  main {
    flex: 1;
    overflow-y: auto;
  }
</style>
