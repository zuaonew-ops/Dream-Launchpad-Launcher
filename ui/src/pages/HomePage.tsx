import {
  actions,
  javaLabel,
  selectLatestReleases,
  selectSelectedVersion,
  useLauncher,
} from "../store";

export default function HomePage() {
  const state = useLauncher();
  const selected = selectSelectedVersion(state);
  const latest = selectLatestReleases(state);
  const installed = state.versions.filter((v) => v.installed).length;
  const running = state.game;

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
        : "安装并准备";

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
        <div className="hero-actions">
          <button type="button" className="ghost" onClick={() => void actions.openDir("data")}>
            打开游戏目录
          </button>
          <button
            type="button"
            className="primary"
            disabled={state.busy || !selected || Boolean(running)}
            onClick={primary}
          >
            {primaryLabel}
          </button>
        </div>
      </header>

      <div className="cards">
        <article className="card">
          <p className="card-label">当前版本</p>
          <p className="card-value">{selected?.id ?? "未选择"}</p>
          <p className="card-hint">
            {running
              ? `运行中 · ${running.id} · pid ${running.pid}`
              : selected?.installed
                ? "已安装，可直接启动"
                : "尚未安装"}
          </p>
        </article>

        <article className="card">
          <p className="card-label">账号</p>
          <p className="card-value">{state.username}</p>
          <p className="card-hint">离线模式 · UUID 本地生成</p>
        </article>

        <article className="card">
          <p className="card-label">Java 运行时</p>
          <p className="card-value">{javaLabel(state.javas)}</p>
          <p className="card-hint">{state.javas.length} 个可用</p>
        </article>
      </div>

      <section className="section">
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

      <section className="panel note">
        <p className="card-label">状态</p>
        <p className="card-hint">{state.notice || state.error || "等待操作"}</p>
      </section>
    </div>
  );
}
