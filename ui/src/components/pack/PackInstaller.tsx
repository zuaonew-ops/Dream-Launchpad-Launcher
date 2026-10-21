import { actions, useLauncher } from "../../store";
import ProgressPanel from "../ProgressPanel";
import { formatBytes } from "../market/format";

/** 加载器键 → 界面上的名字 */
function loaderName(key: string): string {
  if (key.startsWith("fabric")) return "Fabric";
  if (key.startsWith("quilt")) return "Quilt";
  if (key === "neoforge") return "NeoForge";
  if (key === "forge") return "Forge";
  return key;
}

/** 整合包安装：选文件 → 看摘要 → 装成一个新版本 */
export default function PackInstaller() {
  const state = useLauncher();
  const preview = state.packPreview;
  const job = state.install;

  return (
    <section className="panel list-panel">
      <div className="content-tools">
        <button type="button" className="pill" onClick={() => void actions.pickPack()}>
          选择整合包…
        </button>
        <span className="hint">
          支持 Modrinth 的 .mrpack 与 CurseForge 的整合包 zip；装完会成为一个新版本
        </span>
      </div>

      {state.packFile && (
        <div className="pack-file">
          <span className="hint">文件</span>
          <code className="path">{state.packFile}</code>
        </div>
      )}

      {preview && (
        <div className="pack-preview">
          <div className="pack-head">
            <h2>{preview.name}</h2>
            <span className="badge badge-dlp">{preview.format === "mrpack" ? "Modrinth" : "CurseForge"}</span>
          </div>
          <div className="pack-meta">
            <span>
              《我的世界》 <strong>{preview.mc || "未声明"}</strong>
            </span>
            <span>
              加载器{" "}
              <strong>
                {preview.loader ? `${loaderName(preview.loader[0])} ${preview.loader[1]}` : "原版"}
              </strong>
            </span>
            <span>
              文件 <strong>{preview.files}</strong>
            </span>
            <span>
              大小 <strong>{preview.bytes > 0 ? formatBytes(preview.bytes) : "安装时才知道"}</strong>
            </span>
            <span>
              覆盖文件 <strong>{preview.overrides}</strong>
            </span>
          </div>

          {preview.sample.length > 0 && (
            <ul className="pack-sample">
              {preview.sample.map((line) => (
                <li key={line}>{line}</li>
              ))}
              {preview.files > preview.sample.length && (
                <li className="hint">…还有 {preview.files - preview.sample.length} 个</li>
              )}
            </ul>
          )}

          <div className="pack-install">
            <label className="field">
              <span className="label">装成哪个版本</span>
              <input
                value={state.packId}
                onChange={(e) => actions.setPackId(e.target.value)}
                placeholder="版本名，例如 Fabulously-Optimized"
              />
            </label>
            <button
              type="button"
              className="pill primary"
              disabled={state.busy || !state.packId.trim()}
              onClick={() => void actions.installPack()}
            >
              {state.busy ? "安装中…" : "安装"}
            </button>
          </div>
        </div>
      )}

      {!preview && !state.packFile && (
        <p className="empty">
          还没有选整合包。点上面的「选择整合包…」挑一个 .mrpack 或 CurseForge 的 zip。
        </p>
      )}

      {job && (
        <ProgressPanel
          stage={job.stage}
          percent={job.percent}
          done={job.done}
          total={job.total}
          bytesDone={job.bytes_done}
          bytesTotal={job.bytes_total}
          current={job.current}
          speed={job.speed}
        />
      )}
    </section>
  );
}
