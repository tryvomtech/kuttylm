// All calls to the Rust backend go through this file.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppPaths,
  ChatSummary,
  DeviceInfo,
  DownloadEvent,
  EngineStatus,
  HfRepoDetail,
  HfSearchPage,
  HfSize,
  HfSort,
  Library,
  ModelList,
  NetworkInfo,
  SavedChat,
  Settings,
} from "./types";

export function getDeviceInfo(): Promise<DeviceInfo> {
  return invoke<DeviceInfo>("get_device_info");
}

export function getModels(): Promise<ModelList> {
  return invoke<ModelList>("get_models");
}

// ---- Downloads ----

/** Starts a download, or resumes a paused one. */
export function startDownload(modelId: string): Promise<void> {
  return invoke("start_download", { modelId });
}

export function pauseDownload(modelId: string): Promise<void> {
  return invoke("pause_download", { modelId });
}

export function cancelDownload(modelId: string): Promise<void> {
  return invoke("cancel_download", { modelId });
}

export function onDownloadEvent(handler: (event: DownloadEvent) => void): Promise<UnlistenFn> {
  return listen<DownloadEvent>("download-progress", (e) => handler(e.payload));
}

// ---- Downloaded models ----

export function listLocalModels(): Promise<Library> {
  return invoke<Library>("list_local_models");
}

export function deleteModel(modelId: string): Promise<void> {
  return invoke("delete_model", { modelId });
}

// ---- Model engine ----

export function getEngineStatus(): Promise<EngineStatus> {
  return invoke<EngineStatus>("get_engine_status");
}

/** Starts the model. Progress arrives as `engine-status` events. */
export function startModel(modelId: string): Promise<void> {
  return invoke("start_model", { modelId });
}

export function stopModel(): Promise<void> {
  return invoke("stop_model");
}

export function onEngineStatus(handler: (status: EngineStatus) => void): Promise<UnlistenFn> {
  return listen<EngineStatus>("engine-status", (e) => handler(e.payload));
}

// ---- Settings ----

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

/** Checks and saves. Returns the cleaned settings, or throws a friendly message. */
export function saveSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("save_settings", { settings });
}

export function getAppPaths(): Promise<AppPaths> {
  return invoke<AppPaths>("get_app_paths");
}

export function getNetworkInfo(): Promise<NetworkInfo> {
  return invoke<NetworkInfo>("get_network_info");
}

// ---- Saved chats ----

export function listChats(): Promise<ChatSummary[]> {
  return invoke<ChatSummary[]>("list_chats");
}

export function getChat(id: string): Promise<SavedChat> {
  return invoke<SavedChat>("get_chat", { id });
}

export function saveChat(chat: SavedChat): Promise<void> {
  return invoke("save_chat", { chat });
}

export function deleteChat(id: string): Promise<void> {
  return invoke("delete_chat", { id });
}

export function deleteAllChats(): Promise<void> {
  return invoke("delete_all_chats");
}

// ---- Hugging Face ----

export function hfSearch(query: string, sort: HfSort, size: HfSize, cursor: string | null): Promise<HfSearchPage> {
  return invoke<HfSearchPage>("hf_search", { query, sort, size, cursor });
}

/** Lists the versions of one repository. They can then be downloaded by id. */
export function hfRepoFiles(repo: string): Promise<HfRepoDetail> {
  return invoke<HfRepoDetail>("hf_repo_files", { repo });
}

/** Rust errors arrive as plain strings with a friendly message. */
export function errorText(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return "Something went wrong. Please try again.";
}
