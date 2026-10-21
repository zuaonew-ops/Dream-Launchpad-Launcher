interface Props {
  stage: string;
  percent: number;
  done: number;
  total: number;
  /** 已收字节 / 总字节（0 = 不按字节） */
  bytesDone?: number;
  bytesTotal?: number;
  /** 当前正在下载的文件 */
  current?: string;
  /** 速度，形如 "3.2 MB/s" */
  speed?: string;
}

function human(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}

export default function ProgressPanel({
  stage,
  percent,
  done,
  total,
  bytesDone = 0,
  bytesTotal = 0,
  current = "",
  speed = "",
}: Props) {
  const byBytes = bytesTotal > 0;

  return (
    <div className="progress">
      <div className="progress-head">
        <span className="progress-stage">{stage}</span>
        {byBytes ? (
          <span className="progress-count">
            {human(bytesDone)} / {human(bytesTotal)}
          </span>
        ) : (
          total > 0 && (
            <span className="progress-count">
              {done} / {total}
            </span>
          )
        )}
        {speed && <span className="progress-speed">{speed}</span>}
        <span className="progress-pct">{Math.round(percent * 100)}%</span>
      </div>
      <div className="progress-track">
        <div className="progress-fill" style={{ width: `${Math.max(3, percent * 100)}%` }} />
      </div>
      {current && (
        <div className="progress-file" title={current}>
          {current}
        </div>
      )}
    </div>
  );
}
