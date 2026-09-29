// Types that mirror the Rust structs sent over Tauri commands.

export type GpuVendor = "nvidia" | "amd" | "intel" | "apple" | "other";

export interface GpuInfo {
  name: string;
  vendor: GpuVendor;
  integrated: boolean;
  vram_total_bytes: number | null;
  vram_free_bytes: number | null;
}

export interface DeviceInfo {
  os_name: string;
  ram_total_bytes: number;
  ram_available_bytes: number;
  cpu_name: string;
  cpu_physical_cores: number | null;
  cpu_threads: number;
  gpus: GpuInfo[];
}

// ---- Model list (see src-tauri/src/catalog) ----

export interface CatalogFile {
  name: string;
  size_bytes: number;
  sha256: string | null;
}

export type FitLabel = "fits_well" | "may_be_slow" | "will_not_fit";

export interface ModelListItem {
  id: string;
  name: string;
  description: string | null;
  hf_repo: string;
  revision: string;
  files: CatalogFile[];
  mmproj: CatalogFile | null;
  quantization: string | null;
  params_b: number | null;
  context_length: number | null;
  license: string | null;
  tags: string[];
  trending: boolean;
  /** All files together, including the vision file. */
  size_bytes: number;
  fit: FitLabel;
}

/** api = online list, cache = last saved copy, builtin = the list inside the app. */
export type ListSource = "api" | "cache" | "builtin";

export interface ModelList {
  models: ModelListItem[];
  source: ListSource;
  fetched_at: number | null;
  /** The online list could not be loaded; this is an older or built-in list. */
  offline: boolean;
  needs_update: boolean;
  skipped: number;
}

// ---- Downloads (see src-tauri/src/download) ----

export type DownloadStatus = "downloading" | "verifying" | "paused" | "done" | "failed" | "cancelled";

export interface DownloadEvent {
  model_id: string;
  status: DownloadStatus;
  downloaded_bytes: number;
  total_bytes: number;
  speed_bps: number;
  eta_secs: number | null;
  error: string | null;
}

// ---- Downloaded models (see src-tauri/src/library.rs) ----

export type LocalStatus = "ready" | "incomplete";

export interface LocalModel extends Omit<ModelListItem, "size_bytes" | "fit"> {
  status: LocalStatus;
  bytes_on_disk: number;
  total_bytes: number;
  saved_at: number;
}

export interface DiskSpace {
  free_bytes: number;
  total_bytes: number;
}

export interface Library {
  models: LocalModel[];
  used_bytes: number;
  disk: DiskSpace | null;
  models_dir: string;
}

// ---- Model engine (see src-tauri/src/engine) ----

export type RunState = "stopped" | "starting" | "running" | "failed";

export interface EngineStatus {
  state: RunState;
  model_id: string | null;
  model_name: string | null;
  base_url: string | null;
  api_key: string | null;
  uses_gpu: boolean | null;
  note: string | null;
  error: string | null;
  log_tail: string[];
  started_at: number | null;
  /** Settings changed after the start; restart the model to use them. */
  needs_restart: boolean;
}

// ---- Settings (see src-tauri/src/settings.rs) ----

export type GpuSetting = "auto" | "gpu" | "cpu" | "custom";

export interface Settings {
  api_enabled: boolean;
  api_port: number;
  api_lan: boolean;
  api_key: string;
  models_dir: string | null;
  gpu: GpuSetting;
  gpu_layers: number;
}

export interface AppPaths {
  default_models_dir: string;
}

export interface NetworkInfo {
  lan_ip: string | null;
}

// ---- Hugging Face search (see src-tauri/src/hf) ----

export type HfSort = "downloads" | "likes" | "updated" | "trending";
export type HfSize = "any" | "tiny" | "small" | "medium" | "large";

export interface HfRepo {
  id: string;
  downloads: number;
  likes: number;
  pipeline_tag: string | null;
  last_modified: string | null;
  license: string | null;
}

export interface HfSearchPage {
  results: HfRepo[];
  next_cursor: string | null;
}

export interface HfVariant extends ModelListItem {
  recommended: boolean;
}

export interface HfRepoDetail {
  repo: string;
  revision: string;
  params_b: number | null;
  context_length: number | null;
  license: string | null;
  vision: boolean;
  variants: HfVariant[];
  has_folder_files: boolean;
}
