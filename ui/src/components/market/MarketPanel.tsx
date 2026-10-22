import { useEffect } from "react";
import Select from "../Select";
import { actions, useLauncher } from "../../store";
import type { MarketKind, MarketSource } from "../../types";
import MarketDetail from "./MarketDetail";
import MarketRow from "./MarketRow";

const kinds: { value: MarketKind; label: string }[] = [
  { value: "mod", label: "模组" },
  { value: "resourcepack", label: "材质" },
  { value: "shader", label: "光影" },
];

const sources: { value: MarketSource; label: string }[] = [
  { value: "modrinth", label: "Modrinth" },
  { value: "curseforge", label: "CurseForge" },
];

/** 加载器：any = 不筛；vanilla 后端会当成「不限」 */
const loaders: { value: string; label: string }[] = [
  { value: "any", label: "不限加载器" },
  { value: "fabric", label: "Fabric" },
  { value: "quilt", label: "Quilt" },
  { value: "forge", label: "Forge" },
  { value: "neoforge", label: "NeoForge" },
];

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

/** 资源市场容器：筛选 + 列表，点开某条进入详情。 */
export default function MarketPanel() {
  const state = useLauncher();

  // 版本下拉：清单里的正式版 + 本地已装的
  const versionOptions = state.versions.filter((v) => v.kind === "release" || v.local).slice(0, 200);

  // 首次进入直接拉一屏热门（版本默认「不指定」）
  useEffect(() => {
    if (state.marketHits.length === 0 && !state.marketLoading && !state.marketOpen) {
      void actions.searchMarket();
    }
    // 只在首次挂载时触发
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (state.marketOpen) return <MarketDetail />;

  // 没指定版本时装到当前选中的版本（市场页默认「不指定」，不能因此禁用安装）
  const installTarget = state.marketMc || state.selected || "";

  return (
    <section className="panel list-panel">
      <div className="market-filters">
        <div className="seg">
          {kinds.map((k) => (
            <button
              key={k.value}
              type="button"
              className={state.marketKind === k.value ? "seg-item active" : "seg-item"}
              onClick={() => {
                actions.setMarketKind(k.value);
                void actions.searchMarket();
              }}
            >
              {k.label}
            </button>
          ))}
        </div>

        <div className="seg">
          {sources.map((s) => (
            <button
              key={s.value}
              type="button"
              className={state.marketSource === s.value ? "seg-item active" : "seg-item"}
              onClick={() => {
                actions.setMarketSource(s.value);
                void actions.searchMarket();
              }}
            >
              {s.label}
            </button>
          ))}
        </div>

        <label className="mini-field">
          <span className="label">游戏版本</span>
          <Select
            size="sm"
            value={state.marketMc}
            options={[
              { value: "", label: "不指定" },
              ...versionOptions.map((v) => ({ value: v.id, label: v.id })),
            ]}
            onChange={(value) => {
              actions.setMarketMc(value);
              void actions.searchMarket();
            }}
          />
        </label>

        <label className="mini-field">
          <span className="label">加载器</span>
          <Select
            size="sm"
            value={state.marketLoader}
            options={loaders.map((l) => ({ value: l.value, label: l.label }))}
            onChange={(value) => {
              actions.setMarketLoader(value);
              void actions.searchMarket();
            }}
          />
        </label>

        <label className="mini-field">
          <span className="label">类别</span>
          <Select
            size="sm"
            value={state.marketCategory}
            options={categories[state.marketKind].map((c) => ({
              value: c.value,
              label: c.label,
            }))}
            onChange={(value) => {
              actions.setMarketCategory(value);
              void actions.searchMarket();
            }}
          />
        </label>

        <div className="market-search">
          <label className="search">
            <input
              value={state.marketQuery}
              placeholder="搜模组（中文也行：钠 / 优化），留空看热门"
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
            className="primary search-btn"
            disabled={state.marketLoading}
            onClick={() => void actions.searchMarket()}
          >
            {state.marketLoading ? "加载中" : "搜索"}
          </button>
        </div>
      </div>

      <p className="hint">
        {installTarget
          ? `安装目标：${installTarget} 的 ${state.marketKind === "mod" ? "mods" : state.marketKind === "shader" ? "shaderpacks" : "resourcepacks"} 目录${state.marketMc ? "" : "（当前选中版本）"}`
          : "先选一个游戏版本，或者直接看热门榜"}
        {state.marketSource === "curseforge" && !state.marketHits.length && "（CurseForge 需要在设置里填 API key）"}
      </p>

      {state.marketHits.length === 0 ? (
        <div className="empty-soft">
          {state.marketLoading ? (
            <p className="hint">正在从 {state.marketSource === "curseforge" ? "CurseForge" : "Modrinth"} 拉取列表…</p>
          ) : (
            <>
              <div className="empty-icon">◍</div>
              <p className="hint">
                {state.error || "没有结果。换个关键词，或者把筛选放宽一点。"}
              </p>
            </>
          )}
        </div>
      ) : (
        <ul className="market-list">
          {state.marketHits.map((hit, index) => (
            <div key={hit.id} style={{ animationDelay: `${Math.min(index, 12) * 22}ms` }}>
              <MarketRow
                hit={hit}
                busy={state.busy || !installTarget}
                onOpen={() => void actions.openMarketProject(hit)}
                onInstall={() => {
                  if (!installTarget) return;
                  void actions.installMarket(hit.id, state.marketKind);
                }}
              />
            </div>
          ))}
        </ul>
      )}
      {state.marketTotal > 0 && (
        <div className="pager">
          <button
            type="button"
            className="ghost"
            disabled={state.marketLoading || state.marketPage <= 1}
            onClick={() => actions.goMarketPage(state.marketPage - 1)}
          >
            上一页
          </button>
          <span className="pager-info">
            第 <strong>{state.marketPage}</strong> / {Math.max(1, Math.ceil(state.marketTotal / 20))} 页
            <em>共 {state.marketTotal} 条</em>
          </span>
          <button
            type="button"
            className="ghost"
            disabled={
              state.marketLoading || state.marketPage >= Math.ceil(state.marketTotal / 20)
            }
            onClick={() => actions.goMarketPage(state.marketPage + 1)}
          >
            下一页
          </button>
        </div>
      )}

    </section>
  );
}
