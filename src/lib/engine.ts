// The model engine status for all screens. Filled by `engine-status` events from Rust.
import { writable } from "svelte/store";
import { getEngineStatus, onEngineStatus } from "./api";
import type { EngineStatus } from "./types";

const STOPPED: EngineStatus = {
  state: "stopped",
  model_id: null,
  model_name: null,
  base_url: null,
  api_key: null,
  uses_gpu: null,
  note: null,
  error: null,
  log_tail: [],
  started_at: null,
  needs_restart: false,
};

export const engine = writable<EngineStatus>(STOPPED);

let started = false;

export function startEngineEvents(): void {
  if (started) return;
  started = true;
  void onEngineStatus((status) => engine.set(status));
  // The status may have changed before the listener was ready.
  getEngineStatus()
    .then((status) => engine.set(status))
    .catch(() => {});
}
