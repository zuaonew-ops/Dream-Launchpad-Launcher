import { useEffect, useState } from "react";
import { actions, currentName, selectSelectedVersion, useLauncher } from "../store";
import type { VersionSummary } from "../types";

/** 版本类型 → 徽章文案与配色键 */
function tagOf(version: VersionSummary | null): { text: string; kind: string } {
  if (!version) return { text: "未选择", kind: "none" };
  if (version.april_fools) return { text: "愚人节", kind: "april" };
  if (version.loader) return { text: version.loader, kind: version.loader.toLowerCase() };
  if (version.local) return { text: "本地", kind: "local" };
  return { text: "原版", kind: "vanilla" };
}

function tailLines(text: string, count: number): string {
  const nl = String.fromCharCode(10);
  const cr = String.fromCharCode(13);
  return text
    .split(nl)
    .map((line) => line.split(cr).join(""))
    .filter((line) => line.trim() !== "")
    .slice(-count)
    .join(nl);
}

export default function HomePage() {
  const state = useLauncher();
  const selected = selectSelectedVersion(state);
  const installed = state.versions.filter((v) => v.installed);
  const running = state.game;
  const empty = !state.loading && installed.length === 0;
  const [name, setName] = useState("");


  // 底部日志条要一段真实输出，顺便验证日志目录可读
  useEffect(() => {
    void actions.loadLogs();
  }, []);

  // 启动用的版本：主页选的默认版本 > 当前选中 > 第一个已安装
  const launchId =
    state.defaultVersion && installed.some((v) => v.id === state.defaultVersion)
      ? state.defaultVersion
      : selected?.id ?? installed[0]?.id ?? "";
  const launchVersion = installed.find((v) => v.id === launchId) ?? null;
  const isRunning = running?.id === launchId;
  const tag = tagOf(launchVersion);

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
        <header className="page-head">
          <div>
            <h1>主页</h1>
            <p className="desc">还没有扫描到可启动的游戏</p>
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

      {state.logText && (
        <section className="log-strip">
          <div className="log-strip-head">
            <h2>最近日志</h2>
            <span className="hint">{state.logName}</span>
            <button
              type="button"
              className="ghost small"
              onClick={() => window.dispatchEvent(new CustomEvent("dlp:nav", { detail: "tools" }))}
            >
              在工具里查看
            </button>
          </div>
          <pre className="log-strip-body">
            {tailLines(state.logText, 3)}
          </pre>
        </section>
      )}

        </section>
      </div>
    );
  }

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>主页</h1>
          <p className="desc">
            离线登录 · 已安装 {installed.length} 个版本 · 清单里共 {state.versions.length} 个可用
          </p>
        </div>
        <span className={state.error ? "pill danger" : "pill"}>
          {state.error || state.notice || (isRunning ? "游戏运行中" : "就绪")}
        </span>
      </header>

      {/* 启动区：一个版本、一个按钮，其余都是次要信息 */}
      <section className="launch">
        <div className="launch-main">
          <div className="launch-title">
            <span className={`badge badge-${tag.kind}`}>{tag.text}</span>
            <h2>{launchId || "未选择版本"}</h2>
          </div>
          <p className="launch-meta">
            <span>账号 {currentName(state)}</span>
            <span className="sep">·</span>
            <span>
              {isRunning
                ? `运行中 · pid ${running?.pid}`
                : launchVersion?.installed
                  ? "文件完整，可以启动"
                  : "还没有安装"}
            </span>
          </p>

          <div className="launch-actions">
            <button
              type="button"
              className="primary big"
              disabled={state.busy || isRunning || !launchId}
              onClick={primary}
            >
              {primaryLabel}
            </button>
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

        <div className="launch-side">
          <label className="field">
            <span className="label">启动版本</span>
            <select
              className="input"
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
          {state.install && (
            <div className="launch-progress">
              <div className="launch-progress-head">
                <span>{state.install.stage}</span>
                <span>{Math.round(state.install.percent * 100)}%</span>
              </div>
              <div className="meter">
                <span style={{ width: `${Math.round(state.install.percent * 100)}%` }} />
              </div>
            </div>
          )}
        </div>
      </section>

      <div className="home-cols">
        <section className="card account-card">
          <div className="section-head">
            <h2>账号</h2>
            <span className="hint">离线 · 本地生成 UUID</span>
          </div>

          <div className="account-list">
            {state.accounts.length === 0 ? (
              <p className="hint">还没有账号，加一个就能启动。</p>
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
                  {account.kind === "msa" && account.skin ? (
                    <img
                      className="account-avatar img"
                      src={`https://mc-heads.net/avatar/${account.id}/32`}
                      alt=""
                    />
                  ) : (
                    <span className="account-avatar">{account.name.slice(0, 1).toUpperCase()}</span>
                  )}
                  <span className="account-name">{account.name}</span>
                  {account.kind === "msa" && <span className="account-tag">正版</span>}
                  {account.id === state.currentAccount && <span className="account-now">当前</span>}
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
            <button type="button" className="ghost msa-btn" onClick={() => void actions.startMsaLogin()}>
              正版登录
            </button>
          </form>

          {state.msa && (
            <div className="msa-panel">
              <div className="msa-head">
                <span className="msa-title">微软账号登录</span>
                <button
                  type="button"
                  className="icon-btn"
                  title="关闭"
                  onClick={actions.closeMsaLogin}
                >
                  ×
                </button>
              </div>

              {state.msa.status === "requesting" ? (
                <p className="msa-wait">
                  <span className="spinner" />
                  {state.msa.message}
                </p>
              ) : state.msa.status === "waiting" ? (
                <>
                  <p className="msa-hint">在浏览器里打开下面的网址，输入这串码：</p>
                  <div className="msa-code">{state.msa.code?.user_code}</div>
                  <div className="msa-actions">
                    <a
                      className="ghost msa-link"
                      href={state.msa.code?.verification_uri}
                      target="_blank"
                      rel="noreferrer"
                    >
                      打开 {state.msa.code?.verification_uri.replace("https://", "")}
                    </a>
                    <span className="msa-wait">
                      <span className="spinner" />
                      {state.msa.message}
                    </span>
                  </div>
                </>
              ) : state.msa.status === "done" ? (
                <p className="msa-ok">{state.msa.message}</p>
              ) : (
                <p className="msg error">{state.msa.message}</p>
              )}
            </div>
          )}
        </section>

        <section className="card versions-card">
          <div className="section-head">
            <h2>已安装版本</h2>
            <span className="hint">点一下设为默认启动</span>
          </div>

          <div className="ver-grid">
            {installed.map((version) => {
              const t = tagOf(version);
              const active = launchId === version.id;
              return (
                <button
                  key={version.id}
                  type="button"
                  className={active ? "ver-row active" : "ver-row"}
                  onClick={() => {
                    actions.setSelected(version.id);
                    void actions.setDefaultVersion(version.id);
                  }}
                >
                  <span className="ver-id">{version.id}</span>
                  <span className={`badge badge-${t.kind}`}>{t.text}</span>
                  {active && <span className="ver-now">默认</span>}
                </button>
              );
            })}
          </div>

          {state.logText && (
            <div className="log-strip">
              <div className="log-strip-head">
                <h2>最近日志</h2>
                <span className="hint">{state.logName}</span>
                <button
                  type="button"
                  className="ghost small"
                  onClick={() =>
                    window.dispatchEvent(new CustomEvent("dlp:nav", { detail: "tools" }))
                  }
                >
                  在工具里查看
                </button>
              </div>
              <pre className="log-strip-body">{tailLines(state.logText, 4)}</pre>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
