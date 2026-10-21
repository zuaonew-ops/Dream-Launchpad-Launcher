import { actions, useLauncher } from "../../store";
import type { MarketVersion } from "../../types";
import { formatCount } from "./format";
import McVersionGroup from "./McVersionGroup";

/** 项目详情：图标 / 简介 / 百科入口 / 版本列表 */
export default function MarketDetail() {
  const state = useLauncher();
  const project = state.marketOpen;
  if (!project) return null;

  const mc = state.marketMc || state.baseVersion || state.selected || "";
  const loader = state.marketLoader;
  const mcmod = project.title_zh
    ? `https://search.mcmod.cn/s?key=${encodeURIComponent(project.title_zh)}`
    : null;

  // 按《我的世界》版本号分组（一个模组版本可能同时支持多个 MC 版本）
  // 只看适配时只保留当前 MC 版本那组；关掉则全部列出
  const groups = groupByMc(state.marketVersions).filter(
    (group) => !state.onlyCompatible || !mc || group.mc === mc
  );

  return (
    <div className="market-detail">
      <div className="detail-head">
        <button type="button" className="ghost back" onClick={actions.closeMarketProject}>
          ← 返回列表
        </button>
        <span className="detail-source">{project.source === "curseforge" ? "CurseForge" : "Modrinth"}</span>
      </div>

      <section className="detail-card">
        {project.icon ? (
          <img className="detail-icon" src={project.icon} alt="" />
        ) : (
          <span className="detail-icon placeholder" />
        )}

        <div className="detail-text">
          <h2>
            {project.title_zh ?? project.title}
            {project.title_zh && <em>{project.title}</em>}
          </h2>
          <p className="detail-desc">{project.description}</p>
          <div className="detail-tags">
            <span className="pill">{formatCount(project.downloads)} 次下载</span>
            {project.categories.slice(0, 4).map((category) => (
              <span className="pill" key={category}>
                {category}
              </span>
            ))}
          </div>
          <div className="detail-actions">
            {mcmod && (
              <a className="ghost link" href={mcmod} target="_blank" rel="noreferrer">
                MC 百科
              </a>
            )}
            {project.site_url && (
              <a className="ghost link" href={project.site_url} target="_blank" rel="noreferrer">
                项目主页
              </a>
            )}
          </div>
        </div>
      </section>

      <section className="detail-card column">
        <div className="section-head">
          <h2>可用版本</h2>
          <label className="check-inline">
            <input
              type="checkbox"
              checked={state.onlyCompatible}
              onChange={(e) => actions.setOnlyCompatible(e.target.checked)}
            />
            只看适配 {mc ? mc : "当前版本"}
            {loader !== "any" && ` / ${loader}`}
          </label>
        </div>

        {state.marketVersionsLoading ? (
          <p className="hint">正在读取版本列表…</p>
        ) : state.marketVersions.length === 0 ? (
          <p className="hint">
            没有可用文件。把上面的「只看适配」取消掉，可以看到这个项目的全部版本。
          </p>
        ) : (
          <ul className="version-list">
            {groups.map((group) => (
              <McVersionGroup
                key={group.mc}
                mc={group.mc}
                versions={group.versions}
                currentMc={mc}
                currentLoader={loader}
                expanded={state.marketExpanded}
                onToggle={actions.toggleMarketVersion}
              />
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

/** 把模组版本按它支持的 MC 版本拆开分组，MC 版本从新到旧排 */
function groupByMc(versions: MarketVersion[]) {
  const map = new Map<string, MarketVersion[]>();
  for (const version of versions) {
    const list = version.game_versions.length > 0 ? version.game_versions : ["不限版本"];
    for (const mc of list) {
      const bucket = map.get(mc) ?? [];
      bucket.push(version);
      map.set(mc, bucket);
    }
  }
  return [...map.entries()]
    .map(([mc, list]) => ({ mc, versions: list }))
    .sort((a, b) => compareMc(b.mc, a.mc));
}

function compareMc(a: string, b: string): number {
  const pa = a.split(/[^0-9]+/).filter(Boolean).map(Number);
  const pb = b.split(/[^0-9]+/).filter(Boolean).map(Number);
  for (let i = 0; i < Math.max(pa.length, pb.length); i += 1) {
    const diff = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (diff !== 0 && !Number.isNaN(diff)) return diff;
  }
  return a.localeCompare(b);
}
