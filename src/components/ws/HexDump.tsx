interface HexDumpProps {
  text: string;
}

function stringToBytes(text: string): number[] {
  const bytes: number[] = [];
  for (let i = 0; i < text.length; i++) {
    bytes.push(text.charCodeAt(i) & 0xff);
  }
  return bytes;
}

function bytesToHex(bytes: number[]): string {
  return bytes.map((b) => b.toString(16).padStart(2, "0")).join(" ");
}

function bytesToAscii(bytes: number[]): string {
  return bytes
    .map((b) => (b >= 32 && b < 127 ? String.fromCharCode(b) : "."))
    .join("");
}

export function HexDump({ text }: HexDumpProps) {
  const bytes = stringToBytes(text);
  const lines: string[] = [];

  for (let i = 0; i < bytes.length; i += 16) {
    const chunk = bytes.slice(i, Math.min(i + 16, bytes.length));
    const hex = bytesToHex(chunk).padEnd(48);
    const ascii = bytesToAscii(chunk);
    const addr = i.toString(16).padStart(8, "0");

    lines.push(`${addr}  ${hex}  ${ascii}`);
  }

  return (
    <pre className="text-xs font-mono leading-relaxed">
      {lines.map((line, i) => (
        <div key={i}>{line}</div>
      ))}
    </pre>
  );
}
