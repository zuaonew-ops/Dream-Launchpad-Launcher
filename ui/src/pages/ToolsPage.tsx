import { actions, useLauncher } from "../store";

export default function ToolsPage() {
  const state = useLauncher();
  const installed = state.versions.filter((v) => v.installed).length;
  const env = state.environment;

  return (
    <div className="page">
      <header className="page-head">
        <h1>工具</h1>
        <p className="desc">目录与运行环境</p>
      </header>

      <section className="panel">
        <h2>目录</h2>
        <div className="buttons">
          <button type="button" className="tool" onClick={() => void actions.openDir("data")}>
            <span className="tool-title">游戏目录</span>
            <span className="tool-hint">存档 / 配置 / mods</span>
          </button>
          <button type="button" className="tool" onClick={() => void actions.openDir("logs")}>
            <span className="tool-title">日志</span>
            <span className="tool-hint">启动器与游戏输出</span>
          </button>
          <button type="button" className="tool" onClick={() => void actions.openDir("versions")}>
            <span className="tool-title">版本目录</span>
            <span className="tool-hint">每个版本一个文件夹</span>
          </button>
        </div>
      </section>

      <section className="panel">
        <h2>环境</h2>
        <dl className="info">
          <div>
            <dt>系统</dt>
            <dd>
              {env?.os} / {env?.arch}
            </dd>
          </div>
          <div>
            <dt>Tauri</dt>
            <dd>{env?.tauri}</dd>
          </div>
          <div>
            <dt>核心库</dt>
            <dd>dlp-core {env?.core}</dd>
          </div>
          <div>
            <dt>版本</dt>
            <dd>
              {installed} 已安装 / {state.versions.length} 可用
            </dd>
          </div>
          <div>
            <dt>Java</dt>
            <dd>{state.javas.length} 个运行时</dd>
          </div>
          <div className="wide">
            <dt>数据目录</dt>
            <dd>{env?.data_dir}</dd>
          </div>
        </dl>
      </section>

      <p className="todo">更多工具（皮肤、日志分析、存档备份）开发中</p>
    </div>
  );
}
