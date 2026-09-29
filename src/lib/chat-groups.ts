// Groups saved chats by date for the history list: newest first.
import type { ChatSummary } from "./types";

export interface ChatGroup {
  label: string;
  chats: ChatSummary[];
}

const DAY = 86_400_000;

function startOfDay(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

/** "Today", "Yesterday", "Previous 7 days", "Previous 30 days", then "September 2026" … */
export function groupChats(chats: ChatSummary[], now: Date = new Date()): ChatGroup[] {
  const today = startOfDay(now);
  const groups = new Map<string, ChatSummary[]>();
  const sorted = [...chats].sort((a, b) => b.updated_at - a.updated_at);

  for (const chat of sorted) {
    const when = chat.updated_at * 1000;
    let label: string;
    if (when >= today) label = "Today";
    else if (when >= today - DAY) label = "Yesterday";
    else if (when >= today - 7 * DAY) label = "Previous 7 days";
    else if (when >= today - 30 * DAY) label = "Previous 30 days";
    else label = new Date(when).toLocaleDateString(undefined, { month: "long", year: "numeric" });
    if (!groups.has(label)) groups.set(label, []);
    groups.get(label)!.push(chat);
  }
  return [...groups].map(([label, list]) => ({ label, chats: list }));
}
