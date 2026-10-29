/** Render conventional TeX delimiters in reports while leaving code and existing dollar math intact. */
export function reportMath(text: string): string {
  return text.replace(
    /(`+)[\s\S]*?\1|(^ {0,3}~{3,})[^\n]*\n[\s\S]*?^ {0,3}~{3,}[ \t]*(?=\n|$)|\$\$[\s\S]*?\$\$|(?<!\\)\$[^\n]*?(?<!\\)\$|(?<!\\)\\\(([\s\S]*?)(?<!\\)\\\)|(?<!\\)\\\[([\s\S]*?)(?<!\\)\\\]/gm,
    (
      match: string,
      code: string | undefined,
      fence: string | undefined,
      inline: string | undefined,
      display: string | undefined,
    ) => {
      if (code || fence) return match;
      if (inline !== undefined) return `$${inline.trim()}$`;
      if (display !== undefined) return `\n\n$$\n${display.trim()}\n$$\n\n`;
      return match;
    },
  );
}
