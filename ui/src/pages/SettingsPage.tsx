import { actions, selectSelectedVersion, useLauncher } from "../store";
import type { VersionSettings } from "../types";

const FALLBACK: VersionSettings = {
  memory_mb: 2048,
  java_path: null,
  isolate: true,
  extra_jvm_args: [],
};

export default function SettingsPage() {
  const state = useLauncher();
  const installed = state.versions.filter((v) => v.installed);
  const selected = selectSelectedVersion(state);
  const key = state.settingsKey || selected?.id || "*";
  const settings = state.settings ?? FALLBACK;

  const save = (patch: Partial<VersionSettings>) => {
    void actions.saveSettings(key, { ...settings, ...patch });
  };

  return (
    <div className="page">
      <header className="page-head">
        <h1>设置</h1>
        <p className="desc">每个版本一套设置，改完立即保存</p>
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

          <label className="switch-row">
            <div>
              <span className="label">浅色主题</span>
              <p className="hint">深色更护眼；切换有过渡动画</p>
            </div>
            <input
              className="switch"
              type="checkbox"
              checked={state.theme === "light"}
              onChange={(e) => actions.setTheme(e.target.checked ? "light" : "dark")}
            />
          </label>
        </section>

        <section className="panel">
          <h2>适用版本</h2>
          <label className="field">
            <span className="label">正在编辑</span>
            <select
              className="input"
              value={key}
              onChange={(e) => void actions.loadSettings(e.target.value)}
            >
              <option value="*">全局默认（所有版本的兜底值）</option>
              {installed.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.id}
                </option>
              ))}
            </select>
          </label>
          <p className="hint">
            {key === "*"
              ? "这里改的是兜底值；某个版本单独设过就以那个为准"
              : `下面的设置只对 ${key} 生效`}
          </p>
        </section>

        <section className="panel">
          <h2>内存与 Java</h2>
          <label className="field">
            <span className="label">内存上限</span>
            <div className="inline">
              <input
                type="range"
                min={1024}
                max={16384}
                step={512}
                value={settings.memory_mb ?? 2048}
                onChange={(e) => save({ memory_mb: Number(e.target.value) })}
              />
              <span className="value">{settings.memory_mb ?? 2048} MB</span>
            </div>
          </label>

          <label className="field">
            <span className="label">Java 运行时</span>
            <select
              className="input"
              value={settings.java_path ?? ""}
              onChange={(e) => save({ java_path: e.target.value || null })}
            >
              <option value="">自动选择（按版本要求）</option>
              {state.javas.map((j) => (
                <option key={j.path} value={j.path}>
                  Java {j.major} · {j.version}
                </option>
              ))}
            </select>
          </label>
        </section>

        <section className="panel">
          <h2>启动</h2>
          <label className="switch-row">
            <div>
              <span className="label">版本隔离</span>
              <p className="hint">每个版本独立存档与 mods（versions/&lt;版本&gt;/）</p>
            </div>
            <input
              className="switch"
              type="checkbox"
              checked={settings.isolate ?? true}
              onChange={(e) => save({ isolate: e.target.checked })}
            />
          </label>

          <label className="field">
            <span className="label">额外 JVM 参数（一行一个）</span>
            <textarea
              className="input area"
              rows={3}
              spellCheck={false}
              placeholder="-XX:+UseG1GC"
              value={settings.extra_jvm_args.join("\n")}
              onChange={(e) =>
                save({
                  extra_jvm_args: e.target.value
                    .split("\n")
                    .map((line) => line.trim())
                    .filter(Boolean),
                })
              }
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
