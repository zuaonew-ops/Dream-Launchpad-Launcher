import { actions, javaLabel, selectLatestReleases, selectSelectedVersion, useLauncher } from "../store";

export default function HomePage() {
  const state = useLauncher();
  const selected = selectSelectedVersion(state);
  const latest = selectLatestReleases(state);
  const installed = state.versions.filter((v) => v.installed).length;
  const running = state.game;
  const empty = !state.loading && installed === 0;

  const primary = () => {
    if (!selected) return;
    if (selected.installed) void actions.launchSelected();
    else void actions.installSelected();
  };

  const primaryLabel = state.install
    ? `${Math.round(state.install.percent * 100)}%`
    : running
      ? "游戏运行中"
      : selected?.installed
        ? "启动游戏"
        : "去下载";

  if (empty) {
    return (
      <div className="page">
        <header className="hero">
          <div>
            <p className="eyebrow">DREAM LAUNCHPAD</p>
            <h1>准备起飞</h1>
            <p className="desc">还没有可启动的游戏</p>
          </div>
        </header>

        <section className="panel empty-state">
          <h2>没有扫描到游戏</h2>
          <p className="hint">
            当前数据目录下的 <code>versions/</code> 是空的。可以：
          </p>
          <ul className="empty-list">
            <li>去「下载」页装一个原版（可顺便加装模组加载器 / OptiFine）</li>
            <li>
              把已有的 <code>.minecraft/versions</code> 整个复制进来
            </li>
          </ul>
          <code className="path">{state.environment?.data_dir ?? "…"}</code>
          <div className="inline">
            <button type="button" className="ghost" onClick={() => void actions.openDir("data")}>
              打开数据目录
            </button>
            <button type="button" className="ghost" onClick={() => void actions.refresh()}>
              重新扫描
            </button>
          </div>
        </section>
      </div>
    );
  }

  return (
    <div className="page">
      <header className="hero">
        <div>
          <p className="eyebrow">DREAM LAUNCHPAD</p>
          <h1>准备起飞</h1>
          <p className="desc">
            离线登录 · 已安装 {installed} 个版本 · 共 {state.versions.length} 个可用
          </p>
        </div>
      </header>

      <div className="home-grid">
        <section className="instance">
          <div className="instance-top">
            <div className="instance-tile">{(selected?.id ?? "?").slice(0, 2).toUpperCase()}</div>
            <div className="instance-name">
              <strong>{selected?.id ?? "未选择版本"}</strong>
              <span>
                {running
                  ? `运行中 · pid ${running.pid}`
                  : selected?.installed
                    ? "已安装，可直接启动"
                    : "尚未安装"}
              </span>
            </div>
          </div>

          <div className="instance-rows">
            <div className="instance-row">
              <span>账号</span>
              <span>{state.username}</span>
            </div>
            <div className="instance-row">
              <span>Java</span>
              <span>{javaLabel(state.javas)}</span>
            </div>
            <div className="instance-row">
              <span>已安装</span>
              <span>
                {installed} / {state.versions.length}
              </span>
            </div>
          </div>

          <div className="instance-actions">
            <button
              type="button"
              className="primary"
              disabled={state.busy || !selected || Boolean(running)}
              onClick={primary}
            >
              {primaryLabel}
            </button>
            <div className="instance-foot">
              <button type="button" className="ghost" onClick={() => void actions.openDir("data")}>
                打开目录
              </button>
              <button
                type="button"
                className="ghost"
                onClick={() => window.dispatchEvent(new CustomEvent("dlp:nav", { detail: "download" }))}
              >
                下载版本
              </button>
            </div>
          </div>
        </section>

        <div className="home-right">
          <section className="card">
            <div className="section-head">
              <h2>推荐版本</h2>
              <span className="hint">最新正式版</span>
            </div>
            <div className="quick">
              {latest.map((version) => (
                <button
                  key={version.id}
                  type="button"
                  className={state.selected === version.id ? "quick-item active" : "quick-item"}
                  onClick={() => actions.setSelected(version.id)}
                >
                  <span className="quick-id">{version.id}</span>
                  <span className={version.installed ? "quick-state on" : "quick-state"}>
                    {version.installed ? "已安装" : "未安装"}
                  </span>
                </button>
              ))}
            </div>
          </section>

          <section className="card">
            <div className="section-head">
              <h2>已安装</h2>
              <span className="hint">点一下切换当前版本</span>
            </div>
            <div className="quick">
              {state.versions
                .filter((v) => v.installed)
                .map((version) => (
                  <button
                    key={version.id}
                    type="button"
                    className={state.selected === version.id ? "quick-item active" : "quick-item"}
                    onClick={() => actions.setSelected(version.id)}
                  >
                    <span className="quick-id">{version.id}</span>
                    <span className="quick-state on">
                      {version.local ? "本地" : "已安装"}
                    </span>
                  </button>
                ))}
            </div>
          </section>

          <section className="panel">
            <div className="section-head">
              <h2>状态</h2>
            </div>
            <p className="card-hint">{state.notice || state.error || "等待操作"}</p>
          </section>
        </div>
      </div>
    </div>
  );
}
