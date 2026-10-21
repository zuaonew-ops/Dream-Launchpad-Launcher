import { useEffect, useState } from "react";
import { actions, selectSelectedVersion, useLauncher } from "../store";

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}

export default function ContentPage() {
  const state = useLauncher();
  const selected = selectSelectedVersion(state);
  const [tab, setTab] = useState<"mods" | "saves">("mods");
  const id = selected?.id ?? "";

  useEffect(() => {
    if (id) void actions.loadContent(id);
  }, [id]);

  if (!selected) {
    return (
      <div className="page">
        <header className="page-head">
          <h1>内容</h1>
        </header>
        <section className="panel empty-state">
          <h2>还没有选中版本</h2>
          <p className="hint">先去「下载」页装一个版本，或从列表里选一个已安装的版本</p>
        </section>
      </div>
    );
  }

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>内容</h1>
          <p className="desc">
            {id}
            {state.baseVersion && state.baseVersion !== id ? ` · ${state.baseVersion}` : ""} · 模组 /
            存档
          </p>
        </div>
        <div className="seg">
          <button
            type="button"
            className={tab === "mods" ? "seg-item active" : "seg-item"}
            onClick={() => setTab("mods")}
          >
            模组 {state.mods.length > 0 && <span className="seg-count">{state.mods.length}</span>}
          </button>
          <button
            type="button"
            className={tab === "saves" ? "seg-item active" : "seg-item"}
            onClick={() => setTab("saves")}
          >
            存档 {state.saves.length > 0 && <span className="seg-count">{state.saves.length}</span>}
          </button>
        </div>
      </header>

      {tab === "mods" && (
        <section className="panel list-panel">
          <div className="content-tools">
            <span className="hint">启用状态靠 .disabled 后缀，和 PCL 一致</span>
            <button
              type="button"
              className="ghost"
              onClick={() => void actions.openVersionDir(id, "mods")}
            >
              打开 mods 目录
            </button>
          </div>

          {state.mods.length === 0 ? (
            <div className="empty-soft">
              <div className="empty-icon">◍</div>
              <p className="hint">
                这个版本还没有模组。到「资源」里搜一个装上，装完会自动出现在这里。
              </p>
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
      )}

      {tab === "saves" && (
        <section className="panel list-panel">
          <div className="content-tools">
            <span className="hint">每个世界一个文件夹，有 level.dat 才算有效</span>
            <button
              type="button"
              className="ghost"
              onClick={() => void actions.openVersionDir(id, "saves")}
            >
              打开存档目录
            </button>
          </div>

          {state.saves.length === 0 ? (
            <div className="empty-soft">
              <p className="hint">还没有存档。进游戏建一个世界，就会出现在这里。</p>
            </div>
          ) : (
            <ul className="content-list">
              {state.saves.map((save, index) => (
                <li
                  key={save.name}
                  className="content-item"
                  style={{ animationDelay: `${Math.min(index, 12) * 25}ms` }}
                >
                  <span className={save.valid ? "dot on" : "dot"} />
                  <span className="content-name">{save.name}</span>
                  <span className="content-meta">{save.modified || "—"}</span>
                  <span className={save.valid ? "badge ok" : "badge"}>
                    {save.valid ? "可读取" : "缺 level.dat"}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}


    </div>
  );
}
