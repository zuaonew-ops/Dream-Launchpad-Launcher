import { actions, useLauncher } from "../store";

export default function SettingsPage() {
  const state = useLauncher();
  const activeJava = state.javas.find((j) => j.path === state.javaPath) ?? null;

  return (
    <div className="page">
      <header className="page-head">
        <h1>设置</h1>
        <p className="desc">修改后立即生效，保存在本地</p>
      </header>

      <div className="grid">
        <section className="panel">
          <h2>账号</h2>
          <label className="field">
            <span className="label">离线昵称</span>
            <input
              className="input"
              value={state.username}
              maxLength={16}
              spellCheck={false}
              onChange={(e) => actions.setUsername(e.target.value)}
            />
          </label>
          <p className="hint">离线登录只在本地生成 UUID（UUID v3），不联网校验</p>
        </section>

        <section className="panel">
          <h2>性能</h2>
          <label className="field">
            <span className="label">内存上限</span>
            <div className="inline">
              <input
                type="range"
                min={1024}
                max={8192}
                step={512}
                value={state.memory}
                onChange={(e) => actions.setMemory(Number(e.target.value))}
              />
              <span className="value">{state.memory} MB</span>
            </div>
          </label>

          <label className="field">
            <span className="label">Java 运行时</span>
            <select
              className="input"
              value={state.javaPath}
              onChange={(e) => actions.setJavaPath(e.target.value)}
            >
              <option value="">自动选择（按版本要求）</option>
              {state.javas.map((j) => (
                <option key={j.path} value={j.path}>
                  Java {j.major} · {j.version}
                </option>
              ))}
            </select>
          </label>
          <p className="hint">
            {activeJava ? `已固定使用 ${activeJava.path}` : "自动模式会按版本要求挑最低可用版本"}
          </p>
        </section>

        <section className="panel">
          <h2>存储</h2>
          <label className="switch-row">
            <div>
              <span className="label">版本隔离</span>
              <p className="hint">每个版本独立存档与 mods（versions/&lt;版本&gt;/）</p>
            </div>
            <input
              className="switch"
              type="checkbox"
              checked={state.isolate}
              onChange={(e) => actions.setIsolate(e.target.checked)}
            />
          </label>

          <div className="field">
            <span className="label">数据目录</span>
            <code className="path">{state.environment?.data_dir ?? "…"}</code>
            <div className="inline">
              <button type="button" className="ghost" onClick={() => void actions.openDir("data")}>
                打开目录
              </button>
              <button type="button" className="ghost" onClick={() => void actions.openDir("logs")}>
                打开日志
              </button>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
