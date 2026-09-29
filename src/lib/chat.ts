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
  const reply: ChatReply = { content: "", reasoning: "", tokensPerSecond: null, finishReason: null };

  let response: Response;
  try {
    response = await fetch(`${o.baseUrl}/chat/completions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        ...(o.apiKey ? { Authorization: `Bearer ${o.apiKey}` } : {}),
      },
      body: JSON.stringify({ messages: o.messages, stream: true }),
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
  error?: { message?: string; type?: string };
}

export function applyChunk(reply: ChatReply, chunk: Chunk): void {
  if (chunk.error) throw new ChatError(errorMessage(chunk.error));
  const choice = chunk.choices?.[0];
  if (choice?.delta?.content) reply.content += choice.delta.content;
  if (choice?.delta?.reasoning_content) reply.reasoning += choice.delta.reasoning_content;
  if (choice?.finish_reason) reply.finishReason = choice.finish_reason;
  if (chunk.timings?.predicted_per_second) reply.tokensPerSecond = chunk.timings.predicted_per_second;
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
