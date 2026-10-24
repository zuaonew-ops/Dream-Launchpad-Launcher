import { actions, useLauncher } from "../../store";
import Select from "../Select";

function formatMemory(mb: number): string {
  if (mb >= 1024) {
    const gb = mb / 1024;
    return `${Number.isInteger(gb) ? gb : gb.toFixed(1)} GB`;
  }
  return `${mb} MB`;
}

/** 内存上限：不超过物理内存的 80% */
function memoryCeiling(totalMb: number): number {
  return Math.max(1024, Math.floor((totalMb * 0.8) / 256) * 256);
}

/**
 * 全局设置：和具体版本无关的那些 —— 主题、下载源、数据目录、
 * 全局内存与 Java 默认值、本机信息。
 *
 * 版本自己的内存 / Java / 隔离 / JVM 参数在「版本设置」里，不在这里。
 */
export default function GlobalSettings() {
  const state = useLauncher();
  // 全局默认值存在 "*" 这个 key 下
  const settings = state.settings ?? {
    memory_mb: null,
    java_path: null,
    isolate: null,
    extra_jvm_args: [],
    game_args: [],
  };
  const ceiling = memoryCeiling(state.system?.total_mb || 16384);
  const memory = Math.min(settings.memory_mb ?? 4096, ceiling);

  const save = (patch: Partial<typeof settings>) => {
    void actions.saveSettings("*", { ...settings, ...patch });
  };

  return (
    <div className="grid">
      <section className="panel">
        <h2>本机</h2>
        <dl className="stat-grid">
          <div>
            <dt>物理内存</dt>
            <dd>{formatMemory(state.system?.total_mb ?? 0)}</dd>
          </div>
          <div>
            <dt>当前可用</dt>
            <dd>{formatMemory(state.system?.available_mb ?? 0)}</dd>
          </div>
          <div>
            <dt>CPU 核心</dt>
            <dd>{state.system?.cores ?? 0}</dd>
          </div>
          <div>
            <dt>已检测 Java</dt>
            <dd>{state.javas.length}</dd>
          </div>
        </dl>
      </section>

      <section className="panel">
        <div className="section-head">
          <h2>下载与市场</h2>
          <span className="hint">全局生效</span>
        </div>
        <label className="field">
          <span className="label">下载源</span>
          <Select
            block
            value={state.appSettings?.download_source ?? "mirror"}
            options={[
              { value: "mirror", label: "镜像优先", hint: "BMCLAPI，国内快" },
              { value: "official", label: "官方直连", hint: "镜像缺新版时用" },
            ]}
            onChange={(value) =>
              actions.setDownloadSource(value === "official" ? "official" : "mirror")
            }
          />
          <span className="hint">镜像只改写大文件地址；拿不到时自动回退官方</span>
        </label>
        <label className="field">
          <span className="label">CurseForge API key</span>
          <input
            className="input"
            type="password"
            placeholder="不填就只能搜 Modrinth"
            value={state.appSettings?.curseforge_api_key ?? ""}
            onChange={(e) => actions.setCurseForgeKey(e.target.value)}
          />
          <span className="hint">
            到 console.curseforge.com 免费申请；填了才能搜 CurseForge 与装它的整合包
          </span>
          <button
            type="button"
            className="ghost small"
            disabled={state.busy}
            onClick={() => void actions.testCurseForgeKey()}
          >
            测一下这个 key
          </button>
        </label>
      </section>

      <section className="panel">
        <h2>外观</h2>
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
        <div className="section-head">
          <h2>内存与 Java</h2>
          <span className="hint">没单独设置的版本都用这里的值</span>
        </div>
        <label className="field">
          <span className="label">内存上限</span>
          <div className="inline">
            <input
              type="range"
              min={512}
              max={ceiling}
              step={256}
              value={memory}
              onChange={(e) => save({ memory_mb: Number(e.target.value) })}
            />
            <span className="value">{formatMemory(memory)}</span>
          </div>
          <div className="memory-meta">
            <span>
              系统共 {formatMemory(state.system?.total_mb ?? 0)}，当前可用{" "}
              {formatMemory(state.system?.available_mb ?? 0)}；上限按 80% 计算
            </span>
            <div className="memory-quick">
              {[2048, 4096, 8192, ceiling].map((mb, index) => (
                <button
                  key={mb}
                  type="button"
                  className={memory === mb ? "chip active" : "chip"}
                  onClick={() => save({ memory_mb: mb })}
                >
                  {index === 3 ? "建议" : formatMemory(mb)}
                </button>
              ))}
            </div>
          </div>
        </label>

        <label className="field">
          <span className="label">Java 运行时</span>
          <Select
            block
            value={settings.java_path ?? ""}
            options={[
              { value: "", label: "自动选择", hint: "按版本要求挑" },
              ...state.javas.map((j) => ({
                value: j.path,
                label: `Java ${j.major}`,
                hint: j.version,
              })),
            ]}
            onChange={(value) => save({ java_path: value || null })}
          />
        </label>

        <div className="field">
          <span className="label">下载 Java（Adoptium JRE，装到 runtime/）</span>
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
      </section>

      <section className="panel">
        <h2>数据</h2>
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
  );
}
