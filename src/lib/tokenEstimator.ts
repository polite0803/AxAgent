/**
 * Lightweight token estimation mirroring the Rust token_counter.rs.
 * Uses character-based heuristics (~15% accuracy for mixed-language text).
 */

function isCjk(code: number): boolean {
  return (
    (code >= 0x4e00 && code <= 0x9fff) ||
    (code >= 0x3400 && code <= 0x4dbf) ||
    (code >= 0xf900 && code <= 0xfaff) ||
    (code >= 0x3000 && code <= 0x303f) ||
    (code >= 0xff00 && code <= 0xffef) ||
    (code >= 0xac00 && code <= 0xd7af) ||
    (code >= 0x3040 && code <= 0x309f) ||
    (code >= 0x30a0 && code <= 0x30ff)
  );
}

export function estimateTokens(text: string): number {
  if (!text) return 0;

  let asciiChars = 0;
  let cjkChars = 0;

  for (let i = 0; i < text.length; i++) {
    if (isCjk(text.charCodeAt(i))) {
      cjkChars++;
    } else {
      asciiChars++;
    }
  }

  const asciiTokens = Math.ceil(asciiChars / 4);
  const cjkTokens = Math.ceil((cjkChars * 2) / 3);
  return asciiTokens + cjkTokens;
}

const PER_MESSAGE_OVERHEAD = 4;

export function estimateMessageTokens(role: string, content: string): number {
  return estimateTokens(role) + estimateTokens(content) + PER_MESSAGE_OVERHEAD;
}
