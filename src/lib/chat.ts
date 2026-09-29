// Talks to the running model through its OpenAI-compatible API, with streaming.
import { SseParser } from "./sse.ts";

export interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface ChatReply {
  content: string;
  /** Some models "think" first; that text arrives separately. */
  reasoning: string;
  tokensPerSecond: number | null;
  /** "stop", "length", or "stopped" when the user pressed Stop. */
  finishReason: string | null;
  /** Tokens the model read plus wrote: the chat's size in its memory after this answer. */
  totalTokens: number | null;
}

export class ChatError extends Error {}

export interface StreamOptions {
  baseUrl: string;
  apiKey: string | null;
  messages: ChatMessage[];
  signal: AbortSignal;
  onUpdate: (reply: ChatReply) => void;
}

/** Sends the chat and calls `onUpdate` for every new piece of the answer. */
export async function streamChat(o: StreamOptions): Promise<ChatReply> {
  const reply: ChatReply = { content: "", reasoning: "", tokensPerSecond: null, finishReason: null, totalTokens: null };

  let response: Response;
  try {
    response = await fetch(`${o.baseUrl}/chat/completions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        ...(o.apiKey ? { Authorization: `Bearer ${o.apiKey}` } : {}),
      },
      body: JSON.stringify({ messages: o.messages, stream: true, stream_options: { include_usage: true } }),
      signal: o.signal,
    });
  } catch (err) {
    if (o.signal.aborted) return { ...reply, finishReason: "stopped" };
    throw new ChatError("Could not reach the model. Check that it is still running in My models.");
  }

  if (!response.ok || !response.body) {
    throw new ChatError(await friendlyHttpError(response));
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  const parser = new SseParser();
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      for (const data of parser.push(decoder.decode(value, { stream: true }))) {
        if (data === "[DONE]") return reply;
        applyChunk(reply, JSON.parse(data));
        o.onUpdate({ ...reply });
      }
    }
  } catch (err) {
    if (o.signal.aborted) return { ...reply, finishReason: "stopped" };
    if (err instanceof ChatError) throw err;
    throw new ChatError("The answer stopped in the middle. The model may have stopped.");
  }
  return reply;
}

interface Chunk {
  choices?: { delta?: { content?: string | null; reasoning_content?: string | null }; finish_reason?: string | null }[];
  timings?: { predicted_per_second?: number };
  usage?: { total_tokens?: number };
  error?: { message?: string; type?: string };
}

export function applyChunk(reply: ChatReply, chunk: Chunk): void {
  if (chunk.error) throw new ChatError(errorMessage(chunk.error));
  const choice = chunk.choices?.[0];
  if (choice?.delta?.content) reply.content += choice.delta.content;
  if (choice?.delta?.reasoning_content) reply.reasoning += choice.delta.reasoning_content;
  if (choice?.finish_reason) reply.finishReason = choice.finish_reason;
  if (chunk.timings?.predicted_per_second) reply.tokensPerSecond = chunk.timings.predicted_per_second;
  if (typeof chunk.usage?.total_tokens === "number") reply.totalTokens = chunk.usage.total_tokens;
}

async function friendlyHttpError(response: Response): Promise<string> {
  if (response.status === 401) {
    return "The model did not accept the app's key. Stop the model and run it again.";
  }
  try {
    const body = (await response.json()) as Chunk;
    if (body.error) return errorMessage(body.error);
  } catch {
    // Not JSON: use the status below.
  }
  return `The model answered with an error (${response.status}).`;
}

export function errorMessage(error: { message?: string; type?: string }): string {
  if (error.type === "exceed_context_size_error") {
    return "This chat is too long for the model's memory. Start a new chat.";
  }
  return error.message ? `The model reported a problem: ${error.message}` : "The model reported a problem.";
}

// ---------- chat titles ----------

const TITLE_MAX = 60;

/** Shortens text to about `max` characters at a word boundary. */
function shorten(text: string, max: number): string {
  if (text.length <= max) return text;
  const cut = text.slice(0, max + 1);
  const space = cut.lastIndexOf(" ");
  return `${(space > max / 2 ? cut.slice(0, space) : text.slice(0, max)).replace(/[\s,;:.-]+$/, "")}…`;
}

/** The fallback title: the first words of the first question. */
/** Small words that should not end a title ("…message for"). */
const WEAK_END = /\s+(a|an|the|and|or|for|to|of|in|on|with|my|your|is|are)$/i;

export function titleFromQuestion(question: string): string {
  let words = question.replace(/\s+/g, " ").trim().split(" ").slice(0, 8).join(" ");
  words = words.replace(/[?.!,:;]+$/, "");
  while (WEAK_END.test(words)) words = words.replace(WEAK_END, "");
  const clean = words.replace(/^[#>*\-\s]+/, "").replace(/[?.!,:;]+$/, "");
  if (!clean) return "New chat";
  return shorten(clean[0].toUpperCase() + clean.slice(1), TITLE_MAX);
}

/**
 * Cleans a title written by the model. Returns null if it is not usable
 * (empty, far too long, or the model started a normal answer instead).
 */
export function cleanTitle(raw: string): string | null {
  const line = raw
    .split("\n")
    .map((l) => l.trim())
    .find((l) => l.length > 0);
  if (!line) return null;
  let t = line
    .replace(/^(chat\s+)?title\s*[:\-–]\s*/i, "")
    .replace(/[*_`#]/g, "")
    .replace(/[.!:;,]+$/, "")
    .replace(/^["'“”‘’«]+|["'“”‘’»]+$/g, "")
    .replace(/[.!:;,]+$/, "")
    .replace(/\s+/g, " ")
    .trim();
  // Not a title: too short, too long, or the start of a list ("2. Try to…").
  // Not a title: too short, too long, a list of things ("Apple, Orange, Lemon, …")
  // or the start of a list ("2. Try to…").
  if (t.length < 2 || t.split(" ").length > 8 || (t.match(/,/g) ?? []).length >= 2 || /^\d+[.)]\s/.test(t)) {
    return null;
  }
  t = t[0].toUpperCase() + t.slice(1);
  return shorten(t, TITLE_MAX);
}

export interface TitleOptions {
  baseUrl: string;
  apiKey: string | null;
  question: string;
}

/** Asks the running model for a short title. Returns null if it fails. */
export async function aiTitle(o: TitleOptions): Promise<string | null> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 30_000);
  try {
    const response = await fetch(`${o.baseUrl}/chat/completions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        ...(o.apiKey ? { Authorization: `Bearer ${o.apiKey}` } : {}),
      },
      body: JSON.stringify({
        stream: false,
        max_tokens: 24,
        temperature: 0.1,
        // Two examples first: even very small models then answer with a title, not a reply.
        messages: [
          { role: "system", content: "Write a 2 to 5 word title for the chat. Reply with the title only." },
          { role: "user", content: "Chat: What is the capital of France?" },
          { role: "assistant", content: "Capital of France" },
          { role: "user", content: "Chat: Explain how rainbows form in simple words." },
          { role: "assistant", content: "How rainbows form" },
          { role: "user", content: `Chat: ${o.question.replace(/\s+/g, " ").slice(0, 400)}` },
        ],
      }),
      signal: controller.signal,
    });
    if (!response.ok) return null;
    const data = (await response.json()) as { choices?: { message?: { content?: string | null } }[] };
    return cleanTitle(data.choices?.[0]?.message?.content ?? "");
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}
