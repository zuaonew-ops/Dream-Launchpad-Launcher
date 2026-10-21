import ProgressPanel from "../components/ProgressPanel";
import VersionList from "../components/VersionList";
import VersionToolbar from "../components/VersionToolbar";
import {
  actions,
  selectFiltered,
  selectGroups,
  selectSelectedVersion,
  useLauncher,
} from "../store";
import type { LoaderKind } from "../types";

const loaders: { value: LoaderKind; label: string }[] = [
  { value: "none", label: "不装加载器" },
  { value: "fabric", label: "Fabric" },
  { value: "quilt", label: "Quilt" },
  { value: "forge", label: "Forge" },
  { value: "neoforge", label: "NeoForge" },
];

export default function DownloadPage() {
  const state = useLauncher();
  const groups = selectGroups(state);
  const total = selectFiltered(state).length;
  const selected = selectSelectedVersion(state);
  const running = state.game?.id === selected?.id;
  const mc = selected?.id ?? "";

  const loaderVersionDisabled = state.loader === "none" || state.loaderOptions.length === 0;

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>下载</h1>
          <p className="desc">扫描 .minecraft 目录与官方清单，可加装模组加载器</p>
        </div>
        <span className="pill">{total} 个版本</span>
      </header>

      <section className="panel list-panel">
        <VersionToolbar
          keyword={state.search}
          filter={state.filter}
          loading={state.loading}
          total={total}
          onKeyword={actions.setSearch}
          onFilter={actions.setFilter}
          onRefresh={() => void actions.refresh()}
        />
        <VersionList
          groups={groups}
          selected={state.selected}
          loading={state.loading}
          onSelect={actions.setSelected}
        />
      </section>

      {selected && (
        <section className="panel options">
          <div className="option">
            <span className="label">模组加载器</span>
            <select
              className="input small"
              value={state.loader}
              onChange={(e) => actions.setLoader(e.target.value as LoaderKind, mc)}
            >
              {loaders.map((l) => (
                <option key={l.value} value={l.value}>
                  {l.label}
                </option>
              ))}
            </select>
          </div>

          <div className="option">
            <span className="label">加载器版本</span>
            <select
              className="input small"
              value={state.loaderVersion}
              disabled={loaderVersionDisabled}
              onChange={(e) => actions.setLoaderVersion(e.target.value)}
            >
              {state.loaderOptions.length === 0 && <option value="">—</option>}
              {state.loaderOptions.map((c) => (
                <option key={c.version} value={c.version}>
                  {c.label}
                </option>
              ))}
            </select>
          </div>

          <label className="option check">
            <input
              type="checkbox"
              checked={state.optifine}
              onChange={(e) => actions.setOptifine(mc, e.target.checked)}
            />
            <span className="label">高清修复 OptiFine</span>
          </label>

          <div className="option">
            <span className="label">OptiFine 版本</span>
            <select
              className="input small"
              value={state.optifineEdition}
              disabled={!state.optifine || state.optifineOptions.length === 0}
              onChange={(e) => actions.setOptifineEdition(e.target.value)}
            >
              {state.optifineOptions.length === 0 && <option value="">—</option>}
              {state.optifineOptions.map((c) => (
                <option key={c.version} value={c.version}>
                  {c.label}
                </option>
              ))}
            </select>
          </div>
        </section>
      )}

      <footer className="action action-sticky">
        <div className="picked">
          <span className="label">当前选择</span>
          <strong>{selected?.id ?? "未选择"}</strong>
          <span className={selected?.installed ? "badge ok" : "badge"}>
            {selected ? (selected.installed ? "已安装" : "未安装") : "—"}
          </span>
        </div>

        <div className="progress-slot">
          {state.install ? (
            <ProgressPanel
              stage={state.install.stage}
              percent={state.install.percent}
              done={state.install.done}
              total={state.install.total}
            />
          ) : state.error ? (
            <p className="msg error">{state.error}</p>
          ) : state.notice ? (
            <p className="msg ok">{state.notice}</p>
          ) : null}
        </div>

        {selected?.installed && state.loader === "none" && !state.optifine ? (
          <button
            type="button"
            className="primary"
            disabled={state.busy || state.loading || Boolean(running)}
            onClick={() => void actions.launchSelected()}
          >
            {running ? "运行中" : "启动"}
          </button>
        ) : (
          <button
            type="button"
            className="primary"
            disabled={state.busy || state.loading || !selected}
            onClick={() => void actions.installSelected()}
          >
            {selected?.installed ? "安装扩展" : "安装"}
          </button>
        )}
      </footer>
    </div>
  );
}
