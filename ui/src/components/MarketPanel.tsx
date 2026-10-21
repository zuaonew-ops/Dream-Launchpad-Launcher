import { useEffect } from "react";
import { actions, selectSelectedVersion, useLauncher } from "../store";
import type { LoaderKind, MarketKind } from "../types";

const kinds: { value: MarketKind; label: string }[] = [
  { value: "mod", label: "模组" },
  { value: "resourcepack", label: "材质" },
  { value: "shader", label: "光影" },
];

/** 加载器：any = 不筛；vanilla 后端会当成「不限」 */
const loaders: { value: string; label: string }[] = [
  { value: "any", label: "不限加载器" },
  { value: "fabric", label: "Fabric" },
  { value: "quilt", label: "Quilt" },
  { value: "forge", label: "Forge" },
  { value: "neoforge", label: "NeoForge" },
];

/** Modrinth 的分类 id（和官网左侧筛选一致） */
const categories: Record<MarketKind, { value: string; label: string }[]> = {
  mod: [
    { value: "all", label: "全部分类" },
    { value: "optimization", label: "优化" },
    { value: "decoration", label: "装饰" },
    { value: "adventure", label: "冒险" },
    { value: "equipment", label: "装备" },
    { value: "food", label: "食物" },
    { value: "magic", label: "魔法" },
    { value: "technology", label: "科技" },
    { value: "storage", label: "存储" },
    { value: "worldgen", label: "世界生成" },
    { value: "utility", label: "实用工具" },
    { value: "library", label: "依赖库" },
  ],
  resourcepack: [
    { value: "all", label: "全部分类" },
    { value: "realistic", label: "写实" },
    { value: "fantasy", label: "幻想" },
    { value: "medieval", label: "中世纪" },
    { value: "modern", label: "现代" },
    { value: "cartoon", label: "卡通" },
  ],
  shader: [
    { value: "all", label: "全部分类" },
    { value: "realistic", label: "写实" },
    { value: "fantasy", label: "幻想" },
    { value: "cartoon", label: "卡通" },
  ],
};

/** Modrinth 资源搜索与安装。筛选顺序：版本 → 加载器 → 类别。 */
export default function MarketPanel() {
  const state = useLauncher();
  const selected = selectSelectedVersion(state);

  // 版本下拉：清单里的正式版 + 本地已装的
  const versionOptions = state.versions
    .filter((v) => v.kind === "release" || v.local)
    .slice(0, 200);

  useEffect(() => {
    if (!state.marketMc && selected) actions.setMarketMc(selected.id);
  }, [selected, state.marketMc]);

  const noVersion = !state.marketMc;
  const installTarget = state.marketMc;
  const installLoader = state.marketLoader as LoaderKind | "any";

  return (
    <section className="panel list-panel">
      <div className="market-filters">
        <div className="seg">
          {kinds.map((k) => (
            <button
              key={k.value}
              type="button"
              className={state.marketKind === k.value ? "seg-item active" : "seg-item"}
              onClick={() => actions.setMarketKind(k.value)}
            >
              {k.label}
            </button>
          ))}
        </div>

        <label className="mini-field">
          <span className="label">游戏版本</span>
          <select
            className="input small"
            value={state.marketMc}
            onChange={(e) => actions.setMarketMc(e.target.value)}
          >
            <option value="">先选版本</option>
            {versionOptions.map((v) => (
              <option key={v.id} value={v.id}>
                {v.id}
              </option>
            ))}
          </select>
        </label>

        <label className="mini-field">
          <span className="label">加载器</span>
          <select
            className="input small"
            value={state.marketLoader}
            disabled={noVersion}
            onChange={(e) => actions.setMarketLoader(e.target.value)}
          >
            {loaders.map((l) => (
              <option key={l.value} value={l.value}>
                {l.label}
              </option>
            ))}
          </select>
        </label>

        <label className="mini-field">
          <span className="label">类别</span>
          <select
            className="input small"
            value={state.marketCategory}
            disabled={noVersion}
            onChange={(e) => actions.setMarketCategory(e.target.value)}
          >
            {categories[state.marketKind].map((c) => (
              <option key={c.value} value={c.value}>
                {c.label}
              </option>
            ))}
          </select>
        </label>

        <div className="market-search">
          <label className="search">
            <input
              value={state.marketQuery}
              placeholder={noVersion ? "先选游戏版本" : "搜模组（可用中文，如「钠」「优化」）"}
              spellCheck={false}
              disabled={noVersion}
              onChange={(e) => actions.setMarketQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void actions.searchMarket();
              }}
            />
            <span className="count">{state.marketTotal}</span>
          </label>
          <button
            type="button"
            className="primary search-btn"
            disabled={state.marketLoading || noVersion}
            onClick={() => void actions.searchMarket()}
          >
            {state.marketLoading ? "搜索中" : "搜索"}
          </button>
        </div>
      </div>

      <p className="hint">
        {installTarget
          ? `安装目标：${installTarget} 的 mods / resourcepacks / shaderpacks`
          : "先选一个游戏版本，再选加载器与类别"}
      </p>

      {state.marketHits.length === 0 ? (
        <div className="empty-soft">
          <p className="hint">输入关键词搜索，回车即可。中文会按内置词表翻成英文去查。</p>
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
                <span className="content-name">
                  {hit.title_zh ?? hit.title}
                  {hit.title_zh && <span className="title-en">{hit.title}</span>}
                </span>
                <span className="market-desc">{hit.description}</span>
              </span>
              <span className="content-meta">{(hit.downloads / 1000).toFixed(0)}k 下载</span>
              <button
                type="button"
                className="ghost"
                disabled={state.busy || !installTarget}
                onClick={() => void actions.installMarket(hit.id, state.marketKind)}
              >
                安装
              </button>
            </li>
          ))}
        </ul>
      )}
      {installLoader !== "any" && state.marketKind === "mod" && (
        <p className="hint">当前按 {installLoader} 加载器筛选；换加载器会重新搜。</p>
      )}
    </section>
  );
}
