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

export default function DownloadPage() {
  const state = useLauncher();
  const groups = selectGroups(state);
  const total = selectFiltered(state).length;
  const selected = selectSelectedVersion(state);
  const running = state.game?.id === selected?.id;

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>下载</h1>
          <p className="desc">选择版本安装，原版文件经 BMCLAPI 镜像下载</p>
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

      <footer className="action">
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

        {selected?.installed ? (
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
            安装
          </button>
        )}
      </footer>
    </div>
  );
}
