const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** 4370000000 → "4.1 GB" (1 GB = 1024³ bytes, like Ubuntu's system monitor). */
export function formatBytes(bytes: number, decimals = 1): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), UNITS.length - 1);
  const value = bytes / 1024 ** i;
  return `${value.toFixed(i === 0 ? 0 : decimals)} ${UNITS[i]}`;
}

export function percent(part: number, total: number): number {
  if (total <= 0) return 0;
  return Math.max(0, Math.min(100, (part / total) * 100));
}

/** Unix seconds → "27 Sep 2026, 11:40 am" in the user's locale. */
export function formatDateTime(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleString(undefined, {
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

/** Context length in tokens → rough number of English words (1 token ≈ 0.75 words). */
export function approxWords(tokens: number): string {
  const words = Math.round((tokens * 0.75) / 1000) * 1000;
  return words >= 1000 ? `${(words / 1000).toLocaleString()}K words` : `${Math.round(tokens * 0.75)} words`;
}

/** 2_500_000 → "2.4 MB/s". */
export function formatSpeed(bytesPerSec: number): string {
  return `${formatBytes(bytesPerSec)}/s`;
}

/** 3725 → "1 h 2 min", 95 → "1 min 35 s", 8 → "8 s". */
export function formatDuration(secs: number): string {
  const s = Math.max(0, Math.round(secs));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const rest = s % 60;
  if (h > 0) return `${h} h ${m} min`;
  if (m > 0) return `${m} min ${rest} s`;
  return `${rest} s`;
}

/** 53323 → "53K", 1250000 → "1.3M". */
export function formatCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1).replace(/\.0$/, "")}M`;
  if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
  return String(n);
}
