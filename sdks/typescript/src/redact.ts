import { readFileSync, writeFileSync } from "node:fs";

const PATTERNS: Array<[RegExp, string]> = [
  // API keys (sk-...)
  [/(?:sk-[a-zA-Z0-9]{20,})/gi, "[REDACTED_API_KEY]"],
  // API key assignments
  [/(?:api[_-]?key\s*[:=]\s*)['"]?([a-zA-Z0-9_\-]{20,})['"]?/gi, "$&".replace(/['"]?([a-zA-Z0-9_\-]{20,})['"]?/, "[REDACTED]")],
  // Bearer tokens
  [/(bearer\s+)([a-zA-Z0-9_\-.]{20,})/gi, "$1[REDACTED]"],
  // AWS keys
  [/AKIA[0-9A-Z]{16}/g, "[REDACTED_AWS_KEY]"],
  // Passwords
  [/(password\s*[:=]\s*)['"]?([^\s'"]{4,})['"]?/gi, "$1[REDACTED]"],
  // Tokens
  [/(token\s*[:=]\s*)['"]?([a-zA-Z0-9_\-.]{20,})['"]?/gi, "$1[REDACTED]"],
  // Email addresses
  [/[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/g, "[REDACTED_EMAIL]"],
];

export function redact(text: string): string {
  let result = text;
  for (const [pattern, replacement] of PATTERNS) {
    result = result.replace(pattern, replacement);
  }
  return result;
}

export function redactTrajectoryFile(
  inputPath: string,
  outputPath?: string
): void {
  const content = readFileSync(inputPath, "utf-8");
  const redacted = redact(content);
  writeFileSync(outputPath ?? inputPath, redacted);
}
