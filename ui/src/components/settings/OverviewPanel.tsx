import { actions, useLauncher } from "../../store";

/** 版本概况：这个版本是什么、装了什么、放在哪 */
export default function OverviewPanel({ id }: { id: string }) {
  const state = useLauncher();
  const version = state.versions.find((item) => item.id === id);
  const loader = version?.loader;

  const loaderLabel: Record<string, string> = {
    fabric: "Fabric",
    forge: "Forge",
    neoforge: "NeoForge",
    quilt: "Quilt",
    optifine: "OptiFine",
  };

  return (
    <div className="grid">
      <section className="panel">
        <h2>版本</h2>
        <label className="field">
          <span className="label">版本号</span>
          <span className="value">{id}</span>
        </label>
        <label className="field">
          <span className="label">模组加载器</span>
          <span className="value">{loader ? loaderLabel[loader] ?? loader : "原版"}</span>
        </label>
        <label className="field">
          <span className="label">基础版本</span>
          <span className="value">{state.baseVersion || id}</span>
        </label>
        <label className="field">
          <span className="label">安装状态</span>
          <span className="value">
            {version?.installed ? "已安装完整" : version?.local ? "本地目录（非官方清单）" : "未安装"}
          </span>
        </label>
        <label className="field">
          <span className="label">发布时间</span>
          <span className="value">{version?.release_time?.slice(0, 10) ?? "—"}</span>
        </label>
      </section>

      <section className="panel">
        <h2>内容统计</h2>
        <div className="stat-row">
          <div className="stat">
            <span className="stat-num">{state.mods.length}</span>
            <span className="stat-label">模组</span>
          </div>
          <div className="stat">
            <span className="stat-num">{state.saves.length}</span>
            <span className="stat-label">存档</span>
          </div>
          <div className="stat">
            <span className="stat-num">{state.packs.resourcepacks.length}</span>
            <span className="stat-label">资源包</span>
          </div>
          <div className="stat">
            <span className="stat-num">{state.packs.shaderpacks.length}</span>
            <span className="stat-label">光影</span>
          </div>
        </div>

        <label className="field">
          <span className="label">数据目录</span>
          <code className="path">{state.environment?.data_dir ?? "…"}</code>
        </label>

        <div className="inline">
          <button type="button" className="ghost" onClick={() => void actions.openVersionDir(id, "instance")}>
            打开版本目录
          </button>
          <button type="button" className="ghost" onClick={() => void actions.openDir("logs")}>
            打开日志
          </button>
          <button type="button" className="ghost" onClick={() => void actions.cloneVersion(id)}>
            复制版本
          </button>
          <button type="button" className="ghost" onClick={() => void actions.renameVersion(id)}>
            重命名
          </button>
        </div>
      </section>

      <section className="panel">
        <h2>导出与分享</h2>
        <p className="hint">
          把 mods、config、资源包、光影和选项打包成一个 .mrpack —— 别人用「下载 → 整合包」
          就能装出一模一样的版本；存档和日志不会被打包
        </p>
        <div className="inline">
          <button
            type="button"
            className="pill"
            disabled={state.busy}
            onClick={() => void actions.exportPack(id)}
          >
            导出成 .mrpack
          </button>
        </div>
      </section>
    </div>
  );
}
