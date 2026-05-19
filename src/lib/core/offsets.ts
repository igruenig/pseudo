const encoder = new TextEncoder();

export function stringIndexToByteOffset(text: string, index: number): number {
  const boundedIndex = Math.max(0, Math.min(index, text.length));
  return encoder.encode(text.slice(0, boundedIndex)).length;
}

export function byteOffsetToStringIndex(text: string, byteOffset: number): number {
  if (byteOffset <= 0) return 0;

  let bytes = 0;
  for (let index = 0; index < text.length;) {
    if (bytes >= byteOffset) return index;

    const codePoint = text.codePointAt(index);
    if (codePoint === undefined) return text.length;

    const char = String.fromCodePoint(codePoint);
    const nextBytes = bytes + encoder.encode(char).length;
    if (nextBytes > byteOffset) return index;

    bytes = nextBytes;
    index += char.length;
  }

  return text.length;
}

export function byteRangeToStringRange(text: string, start: number, end: number): { start: number; end: number } {
  return {
    start: byteOffsetToStringIndex(text, start),
    end: byteOffsetToStringIndex(text, end)
  };
}
