import { actions, useLauncher } from "../../store";

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}

/** 某个版本的 mods 列表：启用 / 禁用 / 删除 / 打开目录 */
export default function ModPanel({ id }: { id: string }) {
  const state = useLauncher();

  return (
    <section className="panel list-panel">
      <div className="content-tools">
        <span className="hint">启用状态靠 .disabled 后缀，和 PCL 一致</span>
        <div className="inline">
          {state.mods.length > 0 && (
            <>
              <button
                type="button"
                className="ghost"
                onClick={() => void actions.setAllMods(id, true)}
              >
                全部启用
              </button>
              <button
                type="button"
                className="ghost"
                onClick={() => void actions.setAllMods(id, false)}
              >
                全部禁用
              </button>
            </>
          )}
          <button
            type="button"
            className="ghost"
            onClick={() => void actions.openVersionDir(id, "mods")}
          >
            打开 mods 目录
          </button>
        </div>
      </div>

      {state.mods.length === 0 ? (
        <div className="empty-soft">
          <div className="empty-icon">◍</div>
          <p className="hint">这个版本还没有模组。到「下载 → 资源」里搜一个装上，装完会出现在这里。</p>
          <button
            type="button"
            className="ghost"
            onClick={() => {
              actions.setDownloadTab("market");
              window.dispatchEvent(new CustomEvent("dlp:nav", { detail: "download" }));
            }}
          >
            去资源页看看
          </button>
        </div>
      ) : (
        <ul className="content-list">
          {state.mods.map((mod, index) => (
            <li
              key={mod.name}
              className={mod.enabled ? "content-item" : "content-item off"}
              style={{ animationDelay: `${Math.min(index, 12) * 25}ms` }}
            >
              <span className={mod.jar ? "dot on" : "dot"} />
              <span className="content-name" title={mod.name}>
                {mod.name.replace(/\.disabled$/, "")}
              </span>
              <span className="content-meta">{formatSize(mod.size)}</span>
              <label className="mini-switch" title={mod.enabled ? "点一下禁用" : "点一下启用"}>
                <input
                  type="checkbox"
                  checked={mod.enabled}
                  onChange={(e) => void actions.toggleMod(id, mod.name, e.target.checked)}
                />
                <span />
              </label>
              <button
                type="button"
                className="icon-btn"
                title="删除"
                onClick={() => {
                  if (confirm(`删除 ${mod.name}？不可恢复`)) void actions.deleteMod(id, mod.name);
                }}
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
