import { actions, useLauncher } from "../../store";

const kindLabel: Record<string, string> = {
  mod: "模组",
  resourcepack: "资源包",
  shader: "光影",
};

const kindDir: Record<string, string> = {
  mod: "mods",
  resourcepack: "resourcepacks",
  shader: "shaderpacks",
};

/** 安装确认：选目标版本 → 看进度 → 完成 */
export default function InstallDialog() {
  const state = useLauncher();
  const ask = state.installAsk;
  if (!ask) return null;

  const installed = state.versions.filter((v) => v.installed);
  const progress = state.install;
  const running = Boolean(progress) && !state.installDone;
  const percent = progress ? Math.round(progress.percent * 100) : 0;
  const dir = kindDir[ask.kind] ?? "mods";

  return (
    <div className="launch-overlay" role="dialog" aria-modal="true">
      <div className="launch-card install-card">
        <div className="launch-head">
          <span className={state.installDone ? "launch-badge ok" : "launch-badge"}>
            {state.installDone ? "已安装" : running ? "正在安装" : kindLabel[ask.kind] ?? "资源"}
          </span>
          <strong className="install-title" title={ask.title}>
            {ask.title}
          </strong>
        </div>

        {!state.installDone && (
          <>
            <label className="field">
              <span className="label">装到哪个版本</span>
              <select
                className="input"
                value={state.installTargetId}
                disabled={running}
                onChange={(e) => actions.setInstallTargetId(e.target.value)}
              >
                {installed.length === 0 && <option value="">还没有安装任何版本</option>}
                {installed.map((v) => (
                  <option key={v.id} value={v.id}>
                    {v.id}
                  </option>
                ))}
              </select>
            </label>
            <p className="hint">
              目标目录：
              <code>
                {state.installTargetId || "<未选版本>"}/…/{dir}
              </code>
            </p>
          </>
        )}

        {running && (
          <>
            <div className="launch-meter">
              <span style={{ width: `${percent}%` }} />
            </div>
            <p className="hint install-stage">
              {progress?.stage || "准备中"} · {percent}%
              {progress && progress.total > 0 && `（${progress.done}/${progress.total}）`}
            </p>
          </>
        )}

        {state.installDone && (
          <p className="hint install-path">
            已写入 <code>{state.installDone}</code>
          </p>
        )}

        {state.error && <p className="msg error">{state.error}</p>}

        <div className="install-actions">
          {state.installDone ? (
            <button type="button" className="primary" onClick={actions.cancelInstall}>
              好的
            </button>
          ) : (
            <>
              <button type="button" className="ghost" onClick={actions.cancelInstall} disabled={running}>
                取消
              </button>
              <button
                type="button"
                className="primary"
                disabled={running || !state.installTargetId}
                onClick={() => void actions.confirmInstall()}
              >
                {running ? "安装中…" : "安装"}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
