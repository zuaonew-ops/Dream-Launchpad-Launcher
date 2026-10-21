import { useEffect } from "react";
import { actions, useLauncher } from "../store";
import type { VersionSettings } from "../types";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

const EMPTY: VersionSettings = {
  memory_mb: null,
  java_path: null,
  isolate: null,
  extra_jvm_args: [],
};

/** 单个版本的设置面板：内存 / Java / 隔离 / JVM 参数 + 这个版本的模组。
 *  从下载页的版本详情进入，也可以从主页实例卡进来。 */
export default function VersionSettingsPanel({ id }: { id: string }) {
  const state = useLauncher();
  const settings = state.settingsKey === id && state.settings ? state.settings : EMPTY;

  useEffect(() => {
    void actions.loadSettings(id);
    void actions.loadContent(id);
  }, [id]);

  const save = (patch: Partial<VersionSettings>) => {
    void actions.saveSettings(id, { ...settings, ...patch });
  };

  return (
    <div className="settings-grid">
      <section className="panel">
        <h2>内存</h2>
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

        <label className="switch-row">
          <div>
            <span className="label">版本隔离</span>
            <p className="hint">存档与 mods 放在 versions/&lt;版本&gt;/ 下</p>
          </div>
          <input
            className="switch"
            type="checkbox"
            checked={settings.isolate ?? true}
            onChange={(e) => save({ isolate: e.target.checked })}
          />
        </label>
      </section>

      <section className="panel">
        <h2>Java</h2>
        <label className="field">
          <span className="label">运行时</span>
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

        <div className="field">
          <span className="label">装一个（Adoptium JRE）</span>
          <div className="inline">
            {[8, 17, 21, 25].map((major) => (
              <button
                key={major}
                type="button"
                className="ghost"
                disabled={state.busy}
                onClick={() => void actions.installJava(major)}
              >
                Java {major}
              </button>
            ))}
          </div>
        </div>

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

        <div className="inline">
          <button
            type="button"
            className="ghost"
            onClick={() => void actions.openVersionDir(id, "root")}
          >
            打开版本目录
          </button>
          <button
            type="button"
            className="ghost"
            onClick={() => void actions.openVersionDir(id, "instance")}
          >
            打开实例目录
          </button>
        </div>
      </section>

      <section className="panel settings-mods">
        <div className="content-tools">
          <h2>模组</h2>
          <button
            type="button"
            className="ghost"
            onClick={() => void actions.openVersionDir(id, "mods")}
          >
            打开 mods 目录
          </button>
        </div>

        {state.mods.length === 0 ? (
          <p className="hint">这个版本没有模组。去下载页的「资源」里搜一个装上。</p>
        ) : (
          <ul className="content-list">
            {state.mods.map((mod, index) => (
              <li
                key={mod.name}
                className={mod.enabled ? "content-item" : "content-item off"}
                style={{ animationDelay: `${Math.min(index, 12) * 25}ms` }}
              >
                <span className={mod.jar ? "dot on" : "dot"} />
                <span className="content-name" title={mod.name}>
                  {mod.name.replace(/\.disabled$/, "")}
                </span>
                <span className="content-meta">{formatSize(mod.size)}</span>
                <label className="mini-switch" title={mod.enabled ? "点一下禁用" : "点一下启用"}>
                  <input
                    type="checkbox"
                    checked={mod.enabled}
                    onChange={(e) => void actions.toggleMod(id, mod.name, e.target.checked)}
                  />
                  <span />
                </label>
                <button
                  type="button"
                  className="icon-btn"
                  title="删除"
                  onClick={() => {
                    if (confirm(`删除 ${mod.name}？不可恢复`)) void actions.deleteMod(id, mod.name);
                  }}
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
