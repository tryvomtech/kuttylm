// Live download state for all screens. Filled by `download-progress` events from Rust.
import { writable } from "svelte/store";
import { onDownloadEvent } from "./api";
import type { DownloadEvent } from "./types";

/** Latest event per model id. Finished and cancelled downloads are removed. */
export const downloads = writable<Record<string, DownloadEvent>>({});

/** Goes up when the models on disk change, so screens can reload their list. */
export const libraryVersion = writable(0);

let started = false;

export function startDownloadEvents(): void {
  if (started) return;
  started = true;
  void onDownloadEvent((event) => {
    downloads.update((all) => {
      const next = { ...all };
      if (event.status === "done" || event.status === "cancelled") delete next[event.model_id];
      else next[event.model_id] = event;
      return next;
    });
    if (event.status !== "downloading" && event.status !== "verifying") {
      libraryVersion.update((v) => v + 1);
    }
  });
}

export function isActive(event: DownloadEvent | undefined): boolean {
  return event?.status === "downloading" || event?.status === "verifying";
}
