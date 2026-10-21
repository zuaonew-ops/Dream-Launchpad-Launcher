import { useEffect, useState } from "react";
import { actions, selectSelectedVersion, useLauncher } from "../store";
import type { MarketKind } from "../types";

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}

const kinds: { value: MarketKind; label: string }[] = [
  { value: "mod", label: "模组" },
  { value: "resourcepack", label: "材质" },
  { value: "shader", label: "光影" },
];

export default function ContentPage() {
  const state = useLauncher();
  const selected = selectSelectedVersion(state);
  const [tab, setTab] = useState<"mods" | "saves" | "market">("mods");
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
            {id} · {state.baseVersion || "MC"} · 模组 / 存档 / 资源
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
          <button
            type="button"
            className={tab === "market" ? "seg-item active" : "seg-item"}
            onClick={() => setTab("market")}
          >
            资源
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
              <p className="hint">这个版本没有模组。可以去「资源」tab 搜一个装上。</p>
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

      {tab === "market" && (
        <section className="panel list-panel">
          <div className="content-tools market-bar">
            <select
              className="input small"
              value={state.marketKind}
              onChange={(e) => actions.setMarketKind(e.target.value as MarketKind)}
            >
              {kinds.map((k) => (
                <option key={k.value} value={k.value}>
                  {k.label}
                </option>
              ))}
            </select>
            <label className="search">
              <input
                value={state.marketQuery}
                placeholder={`在 Modrinth 搜索${state.baseVersion ? `（适配 ${state.baseVersion}）` : ""}`}
                spellCheck={false}
                onChange={(e) => actions.setMarketQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void actions.searchMarket();
                }}
              />
              <span className="count">{state.marketTotal}</span>
            </label>
            <button
              type="button"
              className="ghost"
              disabled={state.marketLoading}
              onClick={() => void actions.searchMarket()}
            >
              {state.marketLoading ? "搜索中" : "搜索"}
            </button>
          </div>

          {state.marketHits.length === 0 ? (
            <div className="empty-soft">
              <p className="hint">
                输入关键词搜索，回车即可。装好的文件会落在该版本的 mods / resourcepacks /
                shaderpacks 目录。
              </p>
            </div>
          ) : (
            <ul className="content-list">
              {state.marketHits.map((hit, index) => (
                <li
                  key={hit.id}
                  className="content-item market-item"
                  style={{ animationDelay: `${Math.min(index, 12) * 25}ms` }}
                >
                  <span className="dot on" />
                  <span className="market-text">
                    <span className="content-name">{hit.title}</span>
                    <span className="market-desc">{hit.description}</span>
                  </span>
                  <span className="content-meta">
                    {(hit.downloads / 1000).toFixed(0)}k 下载
                  </span>
                  <button
                    type="button"
                    className="ghost"
                    disabled={state.busy}
                    onClick={() => void actions.installMarket(hit.id, state.marketKind)}
                  >
                    安装
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </div>
  );
}
