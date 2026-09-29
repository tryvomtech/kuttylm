// Reads "Server-Sent Events" text as it arrives in pieces.
// llama-server sends one `data: {...}` line per event and ends with `data: [DONE]`.

export class SseParser {
  private partial = "";

  /** Adds a piece of text. Returns the data of every complete `data:` line. */
  push(text: string): string[] {
    this.partial += text;
    const lines = this.partial.split("\n");
    this.partial = lines.pop() ?? "";
    const out: string[] = [];
    for (const raw of lines) {
      const line = raw.endsWith("\r") ? raw.slice(0, -1) : raw;
      if (line.startsWith("data:")) out.push(line.slice(5).trimStart());
    }
    return out;
  }
}
