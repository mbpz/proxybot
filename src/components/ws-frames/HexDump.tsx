interface HexDumpProps {
  payload: string;
  truncated: boolean;
}

/**
 * Render a string as a Latin-1 byte hex dump, 16 bytes per line.
 * Note: the input is already a lossy UTF-8 string (binary frames
 * were converted via String::from_utf8_lossy on the backend).
 */
export function HexDump({ payload, truncated }: HexDumpProps) {
  const lines: string[] = [];
  const bytes: number[] = [];
  for (let i = 0; i < payload.length; i++) {
    bytes.push(payload.charCodeAt(i) & 0xff);
  }
  for (let i = 0; i < bytes.length; i += 16) {
    const chunk = bytes.slice(i, i + 16);
    const hex = chunk
      .map((b) => b.toString(16).padStart(2, "0"))
      .join(" ");
    const ascii = chunk
      .map((b) => (b >= 32 && b < 127 ? String.fromCharCode(b) : "."))
      .join("");
    const offset = i.toString(16).padStart(8, "0");
    lines.push(`${offset}  ${hex.padEnd(48)}  ${ascii}`);
  }
  return (
    <div>
      {truncated && (
        <p className="text-xs text-accent-yellow mb-2">
          Binary frame preview may be lossy. Hex shows first 1KB only.
        </p>
      )}
      <pre className="bg-surface-tertiary rounded p-3 font-mono text-xs overflow-auto">
        {lines.join("\n") || "(empty)"}
      </pre>
    </div>
  );
}
