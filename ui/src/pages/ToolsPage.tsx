import { useEffect } from "react";
import { actions, useLauncher } from "../store";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export default function ToolsPage() {
  const state = useLauncher();
  const installed = state.versions.filter((v) => v.installed).length;
  const env = state.environment;

  useEffect(() => {
    void actions.loadLogs();
    void actions.loadCrashes();
  }, []);

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>工具</h1>
          <p className="desc">目录、日志、崩溃报告与运行环境</p>
        </div>
        <span className="pill">{installed} 个版本已安装</span>
      </header>

      <section className="panel">
        <div className="section-head">
          <h2>目录</h2>
        </div>
        <div className="buttons">
          <button type="button" className="tool" onClick={() => void actions.openDir("data")}>
            <span className="tool-title">游戏目录</span>
            <span className="tool-hint">存档 / 配置 / mods</span>
          </button>
          <button type="button" className="tool" onClick={() => void actions.openDir("logs")}>
            <span className="tool-title">日志目录</span>
            <span className="tool-hint">启动器与游戏输出</span>
          </button>
          <button type="button" className="tool" onClick={() => void actions.openDir("versions")}>
            <span className="tool-title">版本目录</span>
            <span className="tool-hint">每个版本一个文件夹</span>
          </button>
        </div>
      </section>

      <div className="tools-split">
        <section className="panel log-panel">
          <div className="log-head">
            <h2>日志</h2>
            <select
              className="input small"
              value={state.logName}
              onChange={(e) => void actions.loadLogs(e.target.value)}
              hidden={state.logs.length === 0}
            >
              {state.logs.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
          </div>
          <pre className="log-view">{state.logText || "还没有日志。启动一次游戏就会生成。"}</pre>
        </section>

        <section className="panel log-panel">
          <div className="log-head">
            <h2>崩溃报告</h2>
            <button type="button" className="ghost" onClick={() => void actions.loadCrashes()}>
              刷新
            </button>
          </div>

          {state.crashes.length === 0 ? (
            <p className="hint">没有崩溃报告。游戏崩了才会在这里出现。</p>
          ) : (
            <ul className="content-list crash-list">
              {state.crashes.map((crash) => (
                <li
                  key={crash.name}
                  className={state.crashName === crash.name ? "content-item active" : "content-item"}
                >
                  <button
                    type="button"
                    className="crash-row"
                    onClick={() => void actions.readCrash(crash.name)}
                  >
                    <span className="crash-name">{crash.name}</span>
                    <span className="crash-summary">{crash.summary || "（无摘要）"}</span>
                  </button>
                  <span className="content-meta">{formatSize(crash.size)}</span>
                </li>
              ))}
            </ul>
          )}

          {state.crashText && (
            <>
              <div className="log-head">
                <span className="hint">{state.crashName}</span>
              </div>
              <pre className="log-view crash-text">{state.crashText}</pre>
            </>
          )}
        </section>
      </div>

      <section className="panel">
        <div className="section-head">
          <h2>环境</h2>
        </div>
        {env ? (
          <dl className="info">
            <div>
              <dt>系统</dt>
              <dd>
                {env.os} / {env.arch}
              </dd>
            </div>
            <div>
              <dt>核心版本</dt>
              <dd>{env.core}</dd>
            </div>
            <div>
              <dt>Tauri</dt>
              <dd>{env.tauri}</dd>
            </div>
            <div>
              <dt>Java</dt>
              <dd>{state.javas.length} 个</dd>
            </div>
            <div>
              <dt>数据目录</dt>
              <dd>{env.data_dir}</dd>
            </div>
          </dl>
        ) : (
          <p className="hint">读取中…</p>
        )}
      </section>
    </div>
  );
}
