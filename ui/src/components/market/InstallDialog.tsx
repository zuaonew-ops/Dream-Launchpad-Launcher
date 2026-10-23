import Select from "../Select";
import { actions, useLauncher } from "../../store";

const kindLabel: Record<string, string> = {
  mod: "模组",
  resourcepack: "资源包",
  shader: "光影",
};

function humanBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}

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
            {state.installAsk?.kind === "modpack" ? (
              <p className="hint">
                整合包会装成一个**新的**版本（不会动现有版本），装完回主页就能看到
              </p>
            ) : (
            <label className="field">
              <span className="label">装到哪个版本</span>
              <Select
                block
                value={state.installTargetId}
                disabled={running}
                placeholder={installed.length === 0 ? "还没有安装任何版本" : "选一个版本"}
                options={installed.map((v) => ({ value: v.id, label: v.id }))}
                onChange={(value) => actions.setInstallTargetId(value)}
              />
            </label>
            )}
            {state.installAsk?.kind !== "modpack" && (
            <p className="hint">
              目标目录：
              <code>
                {state.installTargetId || "<未选版本>"}/…/{dir}
              </code>
            </p>
            )}
          </>
        )}

        {running && (
          <>
            <div className="launch-meter">
              <span style={{ width: `${percent}%` }} />
            </div>
            <p className="hint install-stage">
              {progress?.stage || "准备中"} · {percent}%
              {progress && progress.bytes_total > 0
                ? `（${humanBytes(progress.bytes_done)} / ${humanBytes(progress.bytes_total)}${progress.speed ? ` · ${progress.speed}` : ""}）`
                : progress && progress.total > 0
                  ? `（${progress.done}/${progress.total}）`
                  : ""}
            </p>
            {progress?.current && (
              <p className="hint install-file" title={progress.current}>
                {progress.current}
              </p>
            )}
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
