import { actions, selectSelectedVersion, useLauncher } from "../store";
import type { MarketKind } from "../types";

const kinds: { value: MarketKind; label: string }[] = [
  { value: "mod", label: "模组" },
  { value: "resourcepack", label: "材质" },
  { value: "shader", label: "光影" },
];

/** Modrinth 资源搜索与安装（0.10.0 放在内容页，0.11.0 起在下载页）。 */
export default function MarketPanel() {
  const state = useLauncher();
  const selected = selectSelectedVersion(state);
  const mc = selected?.id ?? "";

  return (
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

      <p className="hint">
        {mc ? `安装目标：${mc} 的 mods / resourcepacks / shaderpacks` : "先在「版本」里选一个版本"}
      </p>

      {state.marketHits.length === 0 ? (
        <div className="empty-soft">
          <p className="hint">输入关键词搜索，回车即可。</p>
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
              <span className="content-meta">{(hit.downloads / 1000).toFixed(0)}k 下载</span>
              <button
                type="button"
                className="ghost"
                disabled={state.busy || !mc}
                onClick={() => void actions.installMarket(hit.id, state.marketKind)}
              >
                安装
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
