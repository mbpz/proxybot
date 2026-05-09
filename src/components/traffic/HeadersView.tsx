interface HeadersViewProps {
  headers: Record<string, string>;
}

export function HeadersView({ headers }: HeadersViewProps) {
  return (
    <div className="p-4">
      <table className="w-full text-sm">
        <tbody>
          {Object.entries(headers).map(([key, value]) => (
            <tr key={key} className="border-b">
              <td className="font-mono text-gray-600 pr-4 py-1 w-1/3">{key}</td>
              <td className="font-mono py-1 break-all">{value}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
