<script lang="ts">
  import { onMount } from "svelte";
  import Card from "../components/Card.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import CopyText from "../components/CopyText.svelte";
  import Notice from "../components/Notice.svelte";
  import Switch from "../components/Switch.svelte";
  import { errorText, getAppPaths, getNetworkInfo, getSettings, saveSettings, startModel } from "../lib/api";
  import { chatList, removeAllChats, setChatSaving } from "../lib/chat-session";
  import { libraryVersion } from "../lib/downloads";
  import { engine } from "../lib/engine";
  import type { GpuSetting, Settings } from "../lib/types";

  let saved = $state<Settings | null>(null);
  let form = $state<Settings | null>(null);
  let lanIp = $state<string | null>(null);
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let saving = $state(false);
  let showKey = $state(false);
  let defaultModelsDir = $state("");

  onMount(async () => {
    try {
      saved = await getSettings();
      form = { ...saved };
      lanIp = (await getNetworkInfo()).lan_ip;
      defaultModelsDir = (await getAppPaths()).default_models_dir;
    } catch (err) {
      error = errorText(err);
    }
  });

  let changed = $derived(form != null && saved != null && JSON.stringify(form) !== JSON.stringify(saved));
  // Shown addresses follow the saved settings, not the unsaved form.
  let port = $derived(saved?.api_port ?? 8080);

  const gpuChoices: { value: GpuSetting; label: string; help: string }[] = [
    {
      value: "auto",
      label: "Automatic (recommended)",
      help: "Uses a separate graphics card if this computer has one. Otherwise the processor.",
    },
    {
      value: "gpu",
      label: "Always use the graphics card",
      help: "Also uses a graphics card built into the processor. This can be slower.",
    },
    { value: "cpu", label: "Processor only", help: "Never uses the graphics card." },
    {
      value: "custom",
      label: "Custom",
      help: "Puts a fixed number of model layers on the graphics card. For experts.",
    },
  ];

  function makeKey() {
    if (!form) return;
    const bytes = crypto.getRandomValues(new Uint8Array(16));
    form.api_key = "sk-kutty-" + Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
    showKey = true;
  }

  let confirmDeleteChats = $state(false);
  let chatsMessage = $state<string | null>(null);

  async function deleteChats() {
    confirmDeleteChats = false;
    try {
      await removeAllChats();
      chatsMessage = "All chats were deleted.";
    } catch (err) {
      error = errorText(err);
    }
  }

  async function save() {
    if (!form || !saved) return;
    saving = true;
    error = null;
    message = null;
    const folderChanged = (form.models_dir ?? "") !== (saved.models_dir ?? "");
    try {
      saved = await saveSettings({ ...form, models_dir: form.models_dir?.trim() || null });
      form = { ...saved };
      message = "Settings saved.";
      if (folderChanged) libraryVersion.update((v) => v + 1);
      setChatSaving(saved.save_chats);
    } catch (err) {
      error = errorText(err);
    } finally {
      saving = false;
    }
  }

  async function restart() {
    if (!$engine.model_id) return;
    try {
      await startModel($engine.model_id);
    } catch (err) {
      error = errorText(err);
    }
  }
</script>

<div class="page">
  <header>
    <h1>Settings</h1>
  </header>

  {#if $engine.needs_restart}
    <Notice kind="warn">
      <strong>Restart the model to use the new settings.</strong>
      <span>
        {$engine.model_name} still runs with the old settings.
        <button class="btn small" onclick={restart}>Restart {$engine.model_name}</button>
      </span>
    </Notice>
  {/if}

  {#if form}
    <div class="stack">
      <Card title="API — Use the model from other apps">
        <div class="fields">
          <Switch
            bind:checked={form.api_enabled}
            label="Allow other apps to use the model"
            help="Apps that support the OpenAI API can use the running model, for example code editors and chat apps."
          />

          {#if form.api_enabled}
            <label class="field">
              <span class="label">Port</span>
              <input type="number" min="1024" max="65535" bind:value={form.api_port} class="short" />
              <span class="help">A number from 1024 to 65535. Default: 8080.</span>
            </label>

            <Switch
              bind:checked={form.api_lan}
              label="Allow other computers on my network"
              help="Phones and computers on the same Wi-Fi or network can use the model."
            />

            <div class="field">
              <label class="label" for="api-key">API key (optional)</label>
              <div class="row">
                <input
                  id="api-key"
                  type={showKey ? "text" : "password"}
                  bind:value={form.api_key}
                  placeholder="Empty: no key needed"
                  autocomplete="off"
                  spellcheck="false"
                />
                <button type="button" class="btn" onclick={() => (showKey = !showKey)}>
                  {showKey ? "Hide" : "Show"}
                </button>
                <button type="button" class="btn" onclick={makeKey}>Make a key</button>
              </div>
              <span class="help">If you set a key, other apps must send it. KuttyLM's own chat sends it for you.</span>
            </div>

            {#if form.api_lan && !form.api_key.trim()}
              <Notice kind="warn">
                <strong>Anyone on your network can use this model.</strong>
                <span>Set an API key to allow only the apps you choose.</span>
              </Notice>
            {/if}
          {/if}

          {#if saved?.api_enabled}
            <div class="addresses">
              <CopyText label="Address for apps on this computer" value="http://localhost:{port}/v1" />
              {#if saved.api_lan}
                {#if lanIp}
                  <CopyText label="Address for other computers" value="http://{lanIp}:{port}/v1" />
                {:else}
                  <p class="help">This computer is not connected to a network right now.</p>
                {/if}
              {/if}
              <p class="help">
                Model name: any text works. {saved.api_key
                  ? "Send the key as: Authorization: Bearer <your key>."
                  : "No key is needed."}
              </p>
            </div>
          {/if}
        </div>
      </Card>

      <Card title="Graphics card">
        <fieldset class="choices">
          <legend class="sr-only">How the model uses the graphics card</legend>
          {#each gpuChoices as c (c.value)}
            <label class="choice">
              <input type="radio" name="gpu" value={c.value} bind:group={form.gpu} />
              <span>
                <span class="label">{c.label}</span>
                <span class="help">{c.help}</span>
              </span>
            </label>
          {/each}
          {#if form.gpu === "custom"}
            <label class="field indent">
              <span class="label">Layers on the graphics card</span>
              <input type="number" min="0" max="999" bind:value={form.gpu_layers} class="short" />
              <span class="help">More layers are faster, but need more graphics memory.</span>
            </label>
          {/if}
        </fieldset>
        <p class="help">If the graphics card does not work, KuttyLM uses the processor instead.</p>
      </Card>

      <Card title="Models folder">
        <div class="field">
          <label class="label" for="models-dir">Folder for downloaded models</label>
          <div class="row">
            <input
              id="models-dir"
              type="text"
              bind:value={form.models_dir}
              placeholder="Default: {defaultModelsDir}"
              spellcheck="false"
            />
            <button type="button" class="btn" onclick={() => form && (form.models_dir = null)}>Use default</button>
          </div>
          <span class="help">
            Use a full path. Models in the old folder are not moved.
          </span>
        </div>
      </Card>

      <Card title="Chat history">
        <div class="fields">
          <Switch
            bind:checked={form.save_chats}
            label="Save chat history"
            help="Keeps your chats on this computer, so you can open them again later. They are never sent anywhere."
          />
          <div class="row">
            <button
              type="button"
              class="btn"
              disabled={$chatList.length === 0}
              onclick={() => {
                chatsMessage = null;
                confirmDeleteChats = true;
              }}
            >
              Delete all chats
            </button>
            {#if chatsMessage}<span class="saved">{chatsMessage}</span>{/if}
          </div>
        </div>
      </Card>
    </div>

    {#if error}
      <Notice kind="error"><strong>{error}</strong></Notice>
    {/if}

    <div class="actions">
      {#if message && !changed}<span class="saved">{message}</span>{/if}
      <button class="btn" onclick={() => saved && (form = { ...saved })} disabled={!changed || saving}>Undo changes</button>
      <button class="btn btn-primary" onclick={save} disabled={!changed || saving}>
        {saving ? "Saving…" : "Save"}
      </button>
    </div>
  {:else if error}
    <Notice kind="error"><strong>{error}</strong></Notice>
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</div>

<ConfirmDialog
  open={confirmDeleteChats}
  title="Delete all chats?"
  message="All saved chats will be removed from this computer. This cannot be undone."
  confirmLabel="Delete all"
  danger
  onconfirm={deleteChats}
  oncancel={() => (confirmDeleteChats = false)}
/>

<style>
  .page {
    padding: 28px 32px;
    max-width: 860px;
  }

  h1 {
    font-size: 1.5rem;
    margin-bottom: 20px;
  }

  .stack {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label {
    font-weight: 600;
  }

  .help {
    color: var(--text-muted);
    font-size: 0.86rem;
    margin: 0;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  input[type="text"],
  input[type="password"],
  input[type="number"] {
    flex: 1;
    min-width: 0;
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  input:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  input.short {
    flex: none;
    width: 120px;
  }

  .addresses {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }

  .choices {
    border: none;
    margin: 0 0 10px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .choice {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    cursor: pointer;
  }

  .choice input {
    margin-top: 4px;
    accent-color: var(--accent);
  }

  .choice > span {
    display: flex;
    flex-direction: column;
  }

  .indent {
    margin-left: 26px;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  .actions {
    position: sticky;
    bottom: 0;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 10px;
    margin-top: 16px;
    padding: 12px 0;
    background: var(--bg);
  }

  .saved {
    color: var(--good);
    font-size: 0.9rem;
  }

  .btn.small {
    margin-left: 8px;
    padding: 3px 10px;
    font-size: 0.85rem;
  }
</style>
