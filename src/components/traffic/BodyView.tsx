interface BodyViewProps {
  body?: string;
}

export function BodyView({ body }: BodyViewProps) {
  if (!body) {
    return <div className="p-4 text-gray-500">No body content</div>;
  }

  let formattedBody = body;
  try {
    const parsed = JSON.parse(body);
    formattedBody = JSON.stringify(parsed, null, 2);
  } catch {
    // Not JSON, use as-is
  }

  return (
    <pre className="p-4 text-sm font-mono overflow-auto whitespace-pre-wrap">
      {formattedBody}
    </pre>
  );
}
