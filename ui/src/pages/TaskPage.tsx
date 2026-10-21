import { actions, useLauncher } from "../store";

function human(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

/** 下载任务详情：总进度 + 每个阶段各自的进度 */
export default function TaskPage() {
  const state = useLauncher();
  const job = state.install;
  const stages = state.installStages;

  if (!job) {
    return (
      <div className="page">
        <header className="page-head">
          <h1>下载任务</h1>
          <p className="desc">当前没有正在进行或刚结束的下载</p>
        </header>
        <section className="panel empty-state">
          <h2>没有下载任务</h2>
          <p className="hint">在「下载」页选一个版本点安装，进度会出现在这里。</p>
          <button type="button" className="ghost" onClick={actions.backToDownload}>
            ← 去下载页
          </button>
        </section>
      </div>
    );
  }

  const percent = job.percent * 100;
  const active = stages.find((stage) => stage.active) ?? stages[stages.length - 1];

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>下载任务</h1>
          <p className="desc">
            {job.id} · {job.stage}
            {job.speed && ` · ${job.speed}`}
          </p>
        </div>
        <button type="button" className="ghost" onClick={actions.backToDownload}>
          ← 返回下载
        </button>
      </header>

      <section className="panel task-hero">
        <div className="task-big">
          <strong>{percent.toFixed(2)}</strong>
          <span>%</span>
        </div>
        <div className="task-hero-body">
          <div className="progress-track tall">
            <div
              className="progress-fill"
              style={{ width: `${Math.max(1, Math.min(100, percent))}%` }}
            />
          </div>
          <div className="task-hero-meta">
            {job.bytes_total > 0 ? (
              <span>
                {human(job.bytes_done)} / {human(job.bytes_total)}
              </span>
            ) : (
              <span>
                {job.done} / {job.total} 个文件
              </span>
            )}
            {job.speed && <em>{job.speed}</em>}
            {active?.current && <code title={active.current}>{active.current}</code>}
          </div>
        </div>
      </section>

      <section className="panel task-stages">
        <div className="section-head">
          <h2>各阶段进度</h2>
          <span className="hint">{stages.length} 个阶段</span>
        </div>

        {stages.length === 0 ? (
          <p className="hint">还没有阶段信息，等一下下。</p>
        ) : (
          <ul className="stage-list">
            {stages.map((stage) => (
              <li key={stage.name} className={stage.active ? "stage-item active" : "stage-item"}>
                <div className="stage-head">
                  <span className="stage-name">{stage.name}</span>
                  <span className="stage-pct">{(stage.percent * 100).toFixed(2)}%</span>
                </div>
                <div className="progress-track slim">
                  <div
                    className="progress-fill"
                    style={{ width: `${Math.max(0, Math.min(100, stage.percent * 100))}%` }}
                  />
                </div>
                <div className="stage-meta">
                  {stage.bytes_total > 0 ? (
                    <span>
                      {human(stage.bytes_done)} / {human(stage.bytes_total)}
                    </span>
                  ) : (
                    <span>
                      {stage.done} / {stage.total}
                    </span>
                  )}
                  {stage.speed && <em>{stage.speed}</em>}
                  {stage.current && <code title={stage.current}>{stage.current}</code>}
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
