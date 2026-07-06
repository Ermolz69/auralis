import type { MediaStream } from '@/entities/media';

interface StreamsTableProps {
  streams: MediaStream[];
}

export function StreamsTable({ streams }: StreamsTableProps) {
  if (!streams || streams.length === 0) {
    return <div className="text-sm text-muted-foreground">No streams found</div>;
  }

  return (
    <table className="w-full text-sm text-left">
      <tbody>
        {streams.map((s) => (
          <tr key={s.index} className="border-b border-border/50 last:border-0">
            <td className="py-1 text-muted-foreground w-8">{s.index}</td>
            <td className="py-1 font-medium w-24">{s.codec_type.toLowerCase()}</td>
            <td className="py-1">{s.codec_name || s.codec_long_name || 'unknown'}</td>
            <td className="py-1 text-muted-foreground">{s.language || ''}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
