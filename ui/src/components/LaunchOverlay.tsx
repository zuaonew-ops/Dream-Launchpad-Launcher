import { actions, LAUNCH_STEPS, useLauncher } from "../store";

/** 启动时的全屏覆盖层：动画 + 分步进度，直到游戏进程起来。 */
export default function LaunchOverlay() {
  const state = useLauncher();
  const job = state.launching;
  if (!job) return null;

  const done = job.percent >= 1;
  const failed = Boolean(job.error);

  return (
    <div className="launch-overlay" role="status" aria-live="polite">
      <div className={failed ? "launch-card failed" : done ? "launch-card done" : "launch-card"}>
        <div className="launch-head">
          {failed ? (
            <span className="launch-badge error">启动失败</span>
          ) : done ? (
            <span className="launch-badge ok">已启动</span>
          ) : (
            <span className="launch-badge">
              <span className="spinner" />
              正在启动
            </span>
          )}
          <strong>{job.id}</strong>
        </div>

        <div className="launch-meter">
          <span style={{ width: `${Math.round(job.percent * 100)}%` }} />
        </div>

        <ol className="launch-steps">
          {LAUNCH_STEPS.map((label, index) => {
            const state_ =
              failed && index === 0
                ? "failed"
                : index < job.step || done
                  ? "done"
                  : index === job.step
                    ? "active"
                    : "todo";
            return (
              <li key={label} className={`launch-step ${state_}`}>
                <span className="launch-step-dot" />
                {label}
              </li>
            );
          })}
        </ol>

        {failed && <p className="launch-error">{job.error}</p>}

        {(done || failed) && (
          <button type="button" className="ghost" onClick={actions.dismissLaunch}>
            {failed ? "关闭" : "好的"}
          </button>
        )}
      </div>
    </div>
  );
}
