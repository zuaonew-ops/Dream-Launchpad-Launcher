import { useEffect } from "react";
import LoaderPicker from "../components/LoaderPicker";
import MarketPanel from "../components/MarketPanel";
import ProgressPanel from "../components/ProgressPanel";
import VersionList from "../components/VersionList";
import VersionSettingsPanel from "../components/VersionSettingsPanel";
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
  const view = state.downloadView;
  const groups = selectGroups(state);
  const total = selectFiltered(state).length;

  if (view.name === "detail") {
    return <VersionDetail id={view.id} />;
  }

  if (view.name === "settings") {
    return (
      <div className="page">
        <header className="page-head">
          <div>
            <button type="button" className="ghost back" onClick={actions.closeVersion}>
              ← 返回
            </button>
            <h1>版本设置 · {view.id}</h1>
            <p className="desc">只对这个版本生效；改完立刻写入 version-settings.json</p>
          </div>
        </header>
        <VersionSettingsPanel id={view.id} />
      </div>
    );
  }

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>下载</h1>
          <p className="desc">扫本地 .minecraft 与官方清单；点版本进详情再决定装什么</p>
        </div>
        <div className="seg">
          <button
            type="button"
            className={state.downloadTab === "versions" ? "seg-item active" : "seg-item"}
            onClick={() => actions.setDownloadTab("versions")}
          >
            版本
            <span className="seg-count">{total}</span>
          </button>
          <button
            type="button"
            className={state.downloadTab === "market" ? "seg-item active" : "seg-item"}
            onClick={() => actions.setDownloadTab("market")}
          >
            资源
          </button>
        </div>
      </header>

      {state.downloadTab === "versions" ? (
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
            onSelect={(id) => actions.openVersion(id)}
          />
        </section>
      ) : (
        <MarketPanel />
      )}

      {state.install && (
        <section className="action action-sticky">
          <div className="picked">
            <span className="label">正在安装</span>
            <strong>{state.install.id}</strong>
          </div>
          <div className="progress-slot">
            <ProgressPanel
              stage={state.install.stage}
              percent={state.install.percent}
              done={state.install.done}
              total={state.install.total}
            />
          </div>
        </section>
      )}
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} B`;
}

function VersionDetail({ id }: { id: string }) {
  const state = useLauncher();
  const version = selectSelectedVersion(state);
  const installed = version?.installed ?? false;
  const running = state.game?.id === id;
  const installing = state.install?.id === id;

  useEffect(() => {
    void actions.loadVersionStats(id);
  }, [id]);

  const loaderLabel = version?.loader
    ? version.loader
    : version?.local
      ? "本地版本"
      : "原版";

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <button type="button" className="ghost back" onClick={actions.closeVersion}>
            ← 返回列表
          </button>
          <h1>{id}</h1>
          <p className="desc">
            {loaderLabel} · {version?.release_time?.slice(0, 10) || "本地"}
            {version?.april_fools ? " · 愚人节版本" : ""}
          </p>
        </div>
        <div className="hero-actions">
          <button type="button" className="ghost" onClick={() => actions.openVersionSettings(id)}>
            版本设置
          </button>
          <button
            type="button"
            className="ghost"
            onClick={() => void actions.openVersionDir(id, "root")}
          >
            打开目录
          </button>
          {installed && (
            <button
              type="button"
              className="ghost"
              disabled={state.busy}
              title="重新校验文件，缺的会补下"
              onClick={() => void actions.repairVersion(id)}
            >
              校验修复
            </button>
          )}
          {installed && state.loader === "none" && !state.optifine ? (
            <button
              type="button"
              className="primary"
              disabled={state.busy || running}
              onClick={() => void actions.launchSelected()}
            >
              {running ? "运行中" : "启动"}
            </button>
          ) : (
            <button
              type="button"
              className="primary"
              disabled={state.busy || installing}
              onClick={() => void actions.installSelected()}
            >
              {installing ? "安装中…" : installed ? "安装扩展" : "安装"}
            </button>
          )}
        </div>
      </header>

      <section className="panel">
        <div className="section-head">
          <h2>装什么</h2>
          <span className="hint">
            {installed ? "版本已就绪，可以只加装扩展" : "先装原版，再按需加装加载器 / OptiFine"}
          </span>
        </div>
        <LoaderPicker mc={id} />
      </section>


      {installed && (
        <section className="panel">
          <div className="section-head">
            <h2>版本概况</h2>
            <span className="hint">{state.versionStats?.path ?? ""}</span>
          </div>
          <dl className="stat-grid">
            <div>
              <dt>占用空间</dt>
              <dd>{formatBytes(state.versionStats?.size ?? 0)}</dd>
            </div>
            <div>
              <dt>文件数</dt>
              <dd>{state.versionStats?.files ?? 0}</dd>
            </div>
            <div>
              <dt>模组</dt>
              <dd>{state.versionStats?.mods ?? 0}</dd>
            </div>
            <div>
              <dt>存档</dt>
              <dd>{state.versionStats?.saves ?? 0}</dd>
            </div>
          </dl>
        </section>
      )}
      {installed && (
        <section className="panel">
          <div className="section-head">
            <h2>维护</h2>
            <span className="hint">删除后会连同该版本的实例目录一起清掉</span>
          </div>
          <div className="inline">
            <button
              type="button"
              className="ghost danger"
              disabled={state.busy || running}
              onClick={() => {
                if (confirm(`删除版本 ${id}？存档与 mods 也会一起删掉，不可恢复`)) {
                  void actions.deleteVersion(id);
                }
              }}
            >
              删除这个版本
            </button>
          </div>
        </section>
      )}
    </div>
  );
}
