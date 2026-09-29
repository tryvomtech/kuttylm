// The current chat. Lives outside the Chat screen, so an answer keeps
// arriving when the user opens another screen. Kept in memory only.
import { get, writable } from "svelte/store";
import { ChatError, streamChat, type ChatMessage } from "./chat.ts";
import { engine } from "./engine";

export interface ChatEntry {
  id: number;
  role: "user" | "assistant" | "notice";
  content: string;
  reasoning?: string;
  modelName?: string;
  tokensPerSecond?: number | null;
  /** The answer is still arriving. */
  pending?: boolean;
  stopped?: boolean;
  /** "length": the answer was cut at the model's limit. */
  cut?: boolean;
  error?: string;
}

export const entries = writable<ChatEntry[]>([]);
export const busy = writable(false);

let nextId = 1;
let controller: AbortController | null = null;
/** The model of the last answer, to show a note when it changes. */
let lastModelId: string | null = null;

function patch(id: number, change: Partial<ChatEntry>): void {
  entries.update((all) => all.map((e) => (e.id === id ? { ...e, ...change } : e)));
}

/** Messages for the model: user and assistant turns, without errors and notes. */
function history(): ChatMessage[] {
  return get(entries)
    .filter((e) => (e.role === "user" || e.role === "assistant") && !e.error && !e.pending)
    .map((e) => ({ role: e.role as "user" | "assistant", content: e.content }));
}

export async function send(text: string): Promise<void> {
  const message = text.trim();
  const status = get(engine);
  if (!message || get(busy) || status.state !== "running" || !status.base_url) return;

  if (lastModelId && lastModelId !== status.model_id) {
    entries.update((all) => [...all, { id: nextId++, role: "notice", content: `Now talking to ${status.model_name}` }]);
  }
  lastModelId = status.model_id;

  entries.update((all) => [...all, { id: nextId++, role: "user", content: message }]);
  const messages = history();
  const answerId = nextId++;
  entries.update((all) => [
    ...all,
    { id: answerId, role: "assistant", content: "", modelName: status.model_name ?? undefined, pending: true },
  ]);

  busy.set(true);
  controller = new AbortController();
  try {
    const reply = await streamChat({
      baseUrl: status.base_url,
      apiKey: status.api_key,
      messages,
      signal: controller.signal,
      onUpdate: (r) => patch(answerId, { content: r.content, reasoning: r.reasoning }),
    });
    patch(answerId, {
      content: reply.content,
      reasoning: reply.reasoning,
      tokensPerSecond: reply.tokensPerSecond,
      pending: false,
      stopped: reply.finishReason === "stopped",
      cut: reply.finishReason === "length",
    });
  } catch (err) {
    const text = err instanceof ChatError ? err.message : "Something went wrong. Please try again.";
    patch(answerId, { pending: false, error: text });
  } finally {
    controller = null;
    busy.set(false);
  }
}

/** Stops the answer that is arriving. The text so far is kept. */
export function stop(): void {
  controller?.abort();
}

/** Sends the last question again (after an error). */
export function retry(): void {
  const all = get(entries);
  const lastUser = [...all].reverse().find((e) => e.role === "user");
  if (!lastUser) return;
  // Remove the failed answer and the question; `send` adds the question again.
  entries.set(all.filter((e) => e.id < lastUser.id));
  void send(lastUser.content);
}

export function newChat(): void {
  stop();
  entries.set([]);
  lastModelId = null;
}
