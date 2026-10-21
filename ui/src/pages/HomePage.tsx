import { useState } from "react";
import { actions, currentName, selectSelectedVersion, useLauncher } from "../store";

export default function HomePage() {
  const state = useLauncher();
  const selected = selectSelectedVersion(state);
  const installed = state.versions.filter((v) => v.installed);
  const running = state.game;
  const empty = !state.loading && installed.length === 0;
  const [name, setName] = useState("");

  // 启动用的版本：主页选的默认版本 > 当前选中 > 第一个已安装
  const launchId =
    state.defaultVersion && installed.some((v) => v.id === state.defaultVersion)
      ? state.defaultVersion
      : selected?.id ?? installed[0]?.id ?? "";
  const launchVersion = installed.find((v) => v.id === launchId) ?? null;
  const isRunning = running?.id === launchId;

  const primary = () => {
    if (!launchId) return;
    if (launchId !== state.selected) actions.setSelected(launchId);
    if (launchVersion?.installed) void actions.launchSelected();
    else void actions.openVersion(launchId);
  };

  const primaryLabel = state.install
    ? `${Math.round(state.install.percent * 100)}%`
    : isRunning
      ? "游戏运行中"
      : launchVersion?.installed
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
            离线登录 · 已安装 {installed.length} 个版本 · 共 {state.versions.length} 个可用
          </p>
        </div>
      </header>

      <div className="home-grid">
        <div className="home-left-stack">
          <section className="instance">
            <div className="instance-top">
              <div className="instance-tile">{currentName(state).slice(0, 2).toUpperCase()}</div>
              <div className="instance-name">
                <strong>{currentName(state)}</strong>
                <span>离线账号</span>
              </div>
            </div>

            <div className="account-list">
              {state.accounts.length === 0 ? (
                <p className="hint">还没有账号，下面加一个就能启动。</p>
              ) : (
                state.accounts.map((account) => (
                  <div
                    key={account.id}
                    className={
                      account.id === state.currentAccount ? "account-row active" : "account-row"
                    }
                    onClick={() => void actions.useAccount(account.id)}
                    role="button"
                    tabIndex={0}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") void actions.useAccount(account.id);
                    }}
                  >
                    <span className="account-dot" />
                    <span className="account-name">{account.name}</span>
                    <button
                      type="button"
                      className="icon-btn"
                      title="删除账号"
                      onClick={(e) => {
                        e.stopPropagation();
                        void actions.removeAccount(account.id);
                      }}
                    >
                      ×
                    </button>
                  </div>
                ))
              )}
            </div>

            <form
              className="account-add"
              onSubmit={(e) => {
                e.preventDefault();
                const value = name.trim();
                if (!value) return;
                void actions.addAccount(value);
                setName("");
              }}
            >
              <input
                className="input small"
                placeholder="新账号昵称"
                value={name}
                maxLength={16}
                spellCheck={false}
                onChange={(e) => setName(e.target.value)}
              />
              <button type="submit" className="ghost">
                添加
              </button>
            </form>
          </section>

          <section className="instance">
            <div className="instance-top">
              <div className="instance-tile">{(launchId || "?").slice(0, 2).toUpperCase()}</div>
              <div className="instance-name">
                <strong>{launchId || "未选择版本"}</strong>
                <span>
                  {isRunning
                    ? `运行中 · pid ${running?.pid}`
                    : launchVersion?.installed
                      ? "已安装"
                      : "未安装"}
                </span>
              </div>
            </div>

            <label className="field">
              <span className="label">每次启动用这个版本</span>
              <select
                className="input small"
                value={launchId}
                onChange={(e) => {
                  void actions.setDefaultVersion(e.target.value);
                  actions.setSelected(e.target.value);
                }}
              >
                {installed.map((v) => (
                  <option key={v.id} value={v.id}>
                    {v.id}
                  </option>
                ))}
              </select>
            </label>

            <div className="instance-actions">
              <button
                type="button"
                className="primary"
                disabled={state.busy || isRunning || !launchId}
                onClick={primary}
              >
                {primaryLabel}
              </button>
              <div className="instance-foot">
                <button
                  type="button"
                  className="ghost"
                  onClick={() => actions.openVersionSettings(launchId)}
                  disabled={!launchId}
                >
                  版本设置
                </button>
                <button
                  type="button"
                  className="ghost"
                  onClick={() => void actions.openVersionDir(launchId, "root")}
                  disabled={!launchId}
                >
                  打开目录
                </button>
              </div>
            </div>
          </section>
        </div>

        <div className="home-right">
          <section className="card">
            <div className="section-head">
              <h2>已安装</h2>
              <span className="hint">点一下就设为每次启动的版本</span>
            </div>
            <div className="quick">
              {installed.map((version) => (
                <button
                  key={version.id}
                  type="button"
                  className={launchId === version.id ? "quick-item active" : "quick-item"}
                  title={version.id}
                  onClick={() => {
                    actions.setSelected(version.id);
                    void actions.setDefaultVersion(version.id);
                  }}
                >
                  <span className="quick-id">{version.id}</span>
                  <span className="quick-state on">
                    {version.april_fools
                      ? "愚人节"
                      : version.loader ?? (version.local ? "本地" : "原版")}
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
