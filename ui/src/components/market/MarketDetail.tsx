import { actions, useLauncher } from "../../store";
import { formatCount } from "./format";
import VersionFiles from "./VersionFiles";

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
          <span className="hint">
            {mc ? `游戏 ${mc}` : "未选游戏版本"}
            {loader !== "any" && ` · ${loader}`}
          </span>
        </div>

        {state.marketVersionsLoading ? (
          <p className="hint">正在读取版本列表…</p>
        ) : state.marketVersions.length === 0 ? (
          <p className="hint">这个版本 + 加载器组合下没有可用文件，换个版本看看。</p>
        ) : (
          <ul className="version-list">
            {state.marketVersions.map((version) => (
              <VersionFiles
                key={version.version_id}
                version={version}
                mc={mc}
                loader={loader}
                expanded={state.marketExpanded === version.version_id}
                onToggle={() => actions.toggleMarketVersion(version.version_id)}
              />
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
