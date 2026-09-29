// The model's context window ("chat memory"): its size, how much a chat uses,
// fitting long chats into it, and loading a chat into the model before the next question.
//
// Uses llama-server's own endpoints, so the numbers are exact for the running model:
//   GET /props           → context size (n_ctx)
//   POST /apply-template → the chat as the model sees it
//   POST /tokenize       → its length in tokens
//   POST /completion     → with n_predict 0: reads the chat into the model's cache
import type { ChatMessage } from "./chat.ts";

export interface ServerOptions {
  /** OpenAI base URL, e.g. http://127.0.0.1:41234/v1 */
  baseUrl: string;
  apiKey: string | null;
}

function root(baseUrl: string): string {
  return baseUrl.replace(/\/v1\/?$/, "");
}

async function call<T>(o: ServerOptions, path: string, body?: unknown, signal?: AbortSignal): Promise<T> {
  const response = await fetch(`${root(o.baseUrl)}${path}`, {
    method: body === undefined ? "GET" : "POST",
    headers: {
      ...(body === undefined ? {} : { "Content-Type": "application/json" }),
      ...(o.apiKey ? { Authorization: `Bearer ${o.apiKey}` } : {}),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal,
  });
  if (!response.ok) throw new Error(`${path} answered ${response.status}`);
  return (await response.json()) as T;
}

/** The context size of the running model, in tokens. */
export async function contextSize(o: ServerOptions): Promise<number | null> {
  try {
    const props = await call<{ default_generation_settings?: { n_ctx?: number } }>(o, "/props");
    const n = props.default_generation_settings?.n_ctx;
    return typeof n === "number" && n > 0 ? n : null;
  } catch {
    return null;
  }
}

/** Exact number of tokens the model reads for these messages (with its chat template). */
export async function countTokens(o: ServerOptions, messages: ChatMessage[], signal?: AbortSignal): Promise<number> {
  if (messages.length === 0) return 0;
  const { prompt } = await call<{ prompt: string }>(o, "/apply-template", { messages }, signal);
  const { tokens } = await call<{ tokens: unknown[] }>(o, "/tokenize", { content: prompt }, signal);
  return tokens.length;
}

/** Reads the chat into the model (no answer), so the next answer starts fast and knows the chat. */
export async function loadIntoModel(o: ServerOptions, messages: ChatMessage[], signal?: AbortSignal): Promise<void> {
  if (messages.length === 0) return;
  const { prompt } = await call<{ prompt: string }>(o, "/apply-template", { messages }, signal);
  await call(o, "/completion", { prompt, n_predict: 0, cache_prompt: true }, signal);
}

/** Tokens kept free for the answer: a quarter of the context, at most 1024. */
export function answerReserve(size: number): number {
  return Math.min(1024, Math.floor(size / 4));
}

export interface FitResult {
  messages: ChatMessage[];
  /** Tokens of the messages that are kept. */
  tokens: number;
  /** How many of the oldest messages were left out. */
  dropped: number;
}

/**
 * Leaves out the oldest messages until the chat fits into `budget` tokens.
 * The newest message is always kept. `count` measures a list of messages.
 */
export async function fitMessages(
  messages: ChatMessage[],
  budget: number,
  count: (m: ChatMessage[]) => Promise<number>,
): Promise<FitResult> {
  let start = 0;
  let tokens = await count(messages);
  while (tokens > budget && start < messages.length - 1) {
    // Drop a whole question-and-answer pair when possible, so the model does not see half a turn.
    start += messages[start].role === "user" && messages[start + 1]?.role === "assistant" ? 2 : 1;
    if (start > messages.length - 1) start = messages.length - 1;
    tokens = await count(messages.slice(start));
  }
  return { messages: messages.slice(start), tokens, dropped: start };
}
