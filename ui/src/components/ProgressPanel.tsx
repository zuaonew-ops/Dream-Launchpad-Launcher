interface Props {
  stage: string;
  percent: number;
  done: number;
  total: number;
}

export default function ProgressPanel({ stage, percent, done, total }: Props) {
  return (
    <div className="progress">
      <div className="progress-head">
        <span className="progress-stage">{stage}</span>
        {total > 0 && (
          <span className="progress-count">
            {done} / {total}
          </span>
        )}
        <span className="progress-pct">{Math.round(percent * 100)}%</span>
      </div>
      <div className="progress-track">
        <div
          className="progress-fill"
          style={{ width: `${Math.max(3, percent * 100)}%` }}
        />
      </div>
    </div>
  );
}
