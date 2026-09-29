// The current chat and the saved chat history.
//
// The session lives outside the Chat screen, so an answer keeps arriving when the user opens
// another screen. Chats are saved on this computer (unless "Save chat history" is off).
import { get, writable } from "svelte/store";
import { aiTitle, ChatError, streamChat, titleFromQuestion, type ChatMessage } from "./chat.ts";
import { deleteAllChats, deleteChat, getChat, getSettings, listChats, saveChat } from "./api";
import { engine } from "./engine";
import type { ChatSummary, SavedChat, StoredMessage, TitleBy } from "./types";

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

export interface ChatMeta {
  id: string;
  title: string;
  titleBy: TitleBy;
  createdAt: number;
}

export const entries = writable<ChatEntry[]>([]);
export const busy = writable(false);
/** The open chat; `null` until the first question of a new chat. */
export const currentChat = writable<ChatMeta | null>(null);
/** Saved chats, newest first. */
export const chatList = writable<ChatSummary[]>([]);

// Entry ids are never reused, so a late update from an old answer cannot touch another chat.
let nextId = 1;
let controller: AbortController | null = null;
/** The model of the last answer, to show a note when it changes. */
let lastModelId: string | null = null;
let lastModelName: string | null = null;
let saveEnabled: boolean | null = null;

const now = () => Math.floor(Date.now() / 1000);

function newChatId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

async function savingOn(): Promise<boolean> {
  if (saveEnabled === null) {
    try {
      saveEnabled = (await getSettings()).save_chats;
    } catch {
      saveEnabled = true;
    }
  }
  return saveEnabled;
}

/** Called by Settings after "Save chat history" changes. */
export function setChatSaving(on: boolean): void {
  saveEnabled = on;
}

export async function refreshChatList(): Promise<void> {
  try {
    chatList.set(await listChats());
  } catch {
    // The list is not critical; keep the old one.
  }
}

function patch(id: number, change: Partial<ChatEntry>): void {
  entries.update((all) => all.map((e) => (e.id === id ? { ...e, ...change } : e)));
}

/** Messages for the model: user and assistant turns, without errors and notes. */
function history(): ChatMessage[] {
  return get(entries)
    .filter((e) => (e.role === "user" || e.role === "assistant") && !e.error && !e.pending)
    .map((e) => ({ role: e.role as "user" | "assistant", content: e.content }));
}

function toStored(e: ChatEntry): StoredMessage {
  return {
    role: e.role,
    content: e.content,
    reasoning: e.reasoning || null,
    model_name: e.modelName ?? null,
    tokens_per_second: e.tokensPerSecond ?? null,
    stopped: !!e.stopped,
    cut: !!e.cut,
  };
}

function fromStored(m: StoredMessage): ChatEntry {
  return {
    id: nextId++,
    role: m.role,
    content: m.content,
    reasoning: m.reasoning ?? undefined,
    modelName: m.model_name ?? undefined,
    tokensPerSecond: m.tokens_per_second ?? null,
    stopped: m.stopped,
    cut: m.cut,
  };
}

/** Saves the open chat (errors and unfinished answers are left out). */
async function persist(): Promise<void> {
  const meta = get(currentChat);
  if (!meta || !(await savingOn())) return;
  const messages = get(entries)
    .filter((e) => !e.error && !(e.pending && !e.content))
    .map(toStored);
  if (!messages.some((m) => m.role === "user")) return;
  const chat: SavedChat = {
    id: meta.id,
    title: meta.title,
    title_by: meta.titleBy,
    created_at: meta.createdAt,
    updated_at: now(),
    model_id: lastModelId,
    model_name: lastModelName,
    messages,
  };
  try {
    await saveChat(chat);
    await refreshChatList();
  } catch (err) {
    console.error("could not save chat", err);
  }
}

/** Lets the model name the chat after the first answer (unless the user renamed it). */
async function nameChat(chatId: string, question: string, baseUrl: string, apiKey: string | null): Promise<void> {
  const title = await aiTitle({ baseUrl, apiKey, question });
  const meta = get(currentChat);
  if (!title || !meta || meta.id !== chatId || meta.titleBy !== "question") return;
  currentChat.set({ ...meta, title, titleBy: "ai" });
  await persist();
}

export async function send(text: string): Promise<void> {
  const message = text.trim();
  const status = get(engine);
  if (!message || get(busy) || status.state !== "running" || !status.base_url) return;

  let meta = get(currentChat);
  const firstQuestion = !meta;
  if (!meta) {
    meta = { id: newChatId(), title: titleFromQuestion(message), titleBy: "question", createdAt: now() };
    currentChat.set(meta);
  }
  const chatId = meta.id;

  if (lastModelId && lastModelId !== status.model_id) {
    entries.update((all) => [...all, { id: nextId++, role: "notice", content: `Now talking to ${status.model_name}` }]);
  }
  lastModelId = status.model_id;
  lastModelName = status.model_name;

  entries.update((all) => [...all, { id: nextId++, role: "user", content: message }]);
  const messages = history();
  const answerId = nextId++;
  entries.update((all) => [
    ...all,
    { id: answerId, role: "assistant", content: "", modelName: status.model_name ?? undefined, pending: true },
  ]);
  void persist();

  busy.set(true);
  controller = new AbortController();
  let answered = false;
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
    answered = !!reply.content;
  } catch (err) {
    const text = err instanceof ChatError ? err.message : "Something went wrong. Please try again.";
    patch(answerId, { pending: false, error: text });
  } finally {
    controller = null;
    busy.set(false);
  }

  // Only if the user is still in the same chat (they may have opened another one).
  if (get(currentChat)?.id !== chatId) return;
  await persist();
  if (firstQuestion && answered) void nameChat(chatId, message, status.base_url, status.api_key);
}

/** Stops the answer that is arriving. The text so far is kept. */
export function stop(): void {
  controller?.abort();
}

/** Before leaving a chat: stop the answer and save what is there. */
async function leaveCurrent(): Promise<void> {
  if (get(busy)) {
    stop();
    entries.update((all) => all.map((e) => (e.pending ? { ...e, pending: false, stopped: true } : e)));
    await persist();
  }
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

export async function newChat(): Promise<void> {
  await leaveCurrent();
  entries.set([]);
  currentChat.set(null);
  lastModelId = null;
  lastModelName = null;
}

export async function openChat(id: string): Promise<void> {
  if (get(currentChat)?.id === id) return;
  await leaveCurrent();
  const chat = await getChat(id);
  entries.set(chat.messages.map(fromStored));
  currentChat.set({ id: chat.id, title: chat.title, titleBy: chat.title_by || "question", createdAt: chat.created_at });
  lastModelId = chat.model_id;
  lastModelName = chat.model_name;
}

export async function renameChat(id: string, title: string): Promise<void> {
  const clean = title.replace(/\s+/g, " ").trim().slice(0, 120);
  if (!clean) return;
  const meta = get(currentChat);
  if (meta?.id === id) {
    currentChat.set({ ...meta, title: clean, titleBy: "user" });
    await persist();
    return;
  }
  const chat = await getChat(id);
  await saveChat({ ...chat, title: clean, title_by: "user" });
  await refreshChatList();
}

export async function removeChat(id: string): Promise<void> {
  if (get(currentChat)?.id === id) {
    stop();
    entries.set([]);
    currentChat.set(null);
  }
  await deleteChat(id);
  await refreshChatList();
}

export async function removeAllChats(): Promise<void> {
  stop();
  entries.set([]);
  currentChat.set(null);
  await deleteAllChats();
  await refreshChatList();
}
