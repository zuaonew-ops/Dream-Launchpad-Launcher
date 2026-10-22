import { actions, useLauncher } from "../../store";
import type { VersionSettings } from "../../types";
import Select from "../Select";

function formatMemory(mb: number): string {
  if (mb >= 1024) {
    const gb = mb / 1024;
    return `${Number.isInteger(gb) ? gb : gb.toFixed(1)} GB`;
  }
  return `${mb} MB`;
}

function memoryCeiling(totalMb: number): number {
  return Math.max(1024, Math.floor((totalMb * 0.8) / 256) * 256);
}

/** 常用的 JVM 参数预设 */
const JVM_PRESETS: { name: string; desc: string; args: string[] }[] = [
  {
    name: "Aikar's flags",
    desc: "降 GC 停顿，大内存/长时间玩最明显",
    args: [
      "-XX:+UseG1GC",
      "-XX:+ParallelRefProcEnabled",
      "-XX:MaxGCPauseMillis=200",
      "-XX:+UnlockExperimentalVMOptions",
      "-XX:+DisableExplicitGC",
      "-XX:+AlwaysPreTouch",
      "-XX:G1NewSizePercent=30",
      "-XX:G1MaxNewSizePercent=40",
      "-XX:G1HeapRegionSize=8M",
      "-XX:G1ReservePercent=20",
      "-XX:G1HeapWastePercent=5",
      "-XX:G1MixedGCCountTarget=4",
      "-XX:InitiatingHeapOccupancyPercent=15",
      "-XX:G1MixedGCLiveThresholdPercent=90",
      "-XX:G1RSetUpdatingPauseTimePercent=5",
      "-XX:SurvivorRatio=32",
      "-XX:+PerfDisableSharedMem",
      "-XX:MaxTenuringThreshold=1",
    ],
  },
  {
    name: "低配机",
    desc: "内存小、核数少时少开线程",
    args: ["-XX:+UseSerialGC", "-XX:TieredStopAtLevel=1", "-Xss1M"],
  },
];

const FALLBACK: VersionSettings = {
  memory_mb: null,
  java_path: null,
  isolate: null,
  extra_jvm_args: [],
  game_args: [],
};

/**
 * 版本设置：只放「这一个版本」才有的东西。
 *
 * 全局默认值（主题、下载源、默认内存和 Java）在侧边栏「设置」里；
 * 这里的每一项留空就是「跟着全局走」。
 */
export default function VersionSettingsPanel({ id }: { id: string }) {
  const state = useLauncher();
  const settings = state.settings ?? FALLBACK;
  const ceiling = memoryCeiling(state.system?.total_mb || 16384);
  const memory = Math.min(settings.memory_mb ?? 0, ceiling);

  const save = (patch: Partial<VersionSettings>) => {
    void actions.saveSettings(id, { ...settings, ...patch });
  };

  return (
    <div className="grid">
      <section className="panel">
        <div className="section-head">
          <h2>内存</h2>
          <span className="hint">留空就跟着全局</span>
        </div>
        <label className="field">
          <span className="label">
            内存上限 <span className="value">{memory ? formatMemory(memory) : "跟随全局"}</span>
          </span>
          <div className="inline">
            <input
              type="range"
              min={0}
              max={ceiling}
              step={256}
              value={memory}
              onChange={(e) => {
                const next = Number(e.target.value);
                save({ memory_mb: next === 0 ? null : next });
              }}
            />
            <button
              type="button"
              className="ghost small"
              onClick={() => save({ memory_mb: null })}
            >
              跟随全局
            </button>
          </div>
          <div className="memory-meta">
            <span>系统共 {formatMemory(state.system?.total_mb ?? 0)}，上限按 80% 计算</span>
            <div className="memory-quick">
              {[2048, 4096, 8192, ceiling].map((mb, index) => (
                <button
                  key={mb}
                  type="button"
                  className={memory === mb ? "chip active" : "chip"}
                  onClick={() => save({ memory_mb: mb })}
                >
                  {index === 3 ? "上限" : formatMemory(mb)}
                </button>
              ))}
            </div>
          </div>
        </label>
      </section>

      <section className="panel">
        <div className="section-head">
          <h2>Java</h2>
          <span className="hint">这个版本用哪个 Java</span>
        </div>
        <label className="field">
          <span className="label">运行时</span>
          <Select
            block
            value={settings.java_path ?? ""}
            options={[
              { value: "", label: "跟随全局", hint: "按版本要求自动挑" },
              ...state.javas.map((j) => ({
                value: j.path,
                label: `Java ${j.major}`,
                hint: j.version,
              })),
            ]}
            onChange={(value) => save({ java_path: value || null })}
          />
        </label>
      </section>

      <section className="panel">
        <div className="section-head">
          <h2>启动</h2>
          <span className="hint">只影响 {id}</span>
        </div>
        <label className="switch-row">
          <div>
            <span className="label">版本隔离</span>
            <p className="hint">存档与 mods 放在 versions/{id}/ 下，不和别的版本共用</p>
          </div>
          <input
            className="switch"
            type="checkbox"
            checked={settings.isolate ?? true}
            onChange={(e) => save({ isolate: e.target.checked })}
          />
        </label>

        <label className="field">
          <span className="label">
            JVM 参数（一行一个）
            <span className="label-actions">
              {JVM_PRESETS.map((preset) => (
                <button
                  key={preset.name}
                  type="button"
                  className="ghost tiny"
                  title={preset.desc}
                  onClick={() => save({ extra_jvm_args: preset.args })}
                >
                  {preset.name}
                </button>
              ))}
            </span>
          </span>
          <textarea
            className="input area"
            rows={4}
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

        <label className="field">
          <span className="label">游戏参数（拼在主类后面，比如 --width 1280）</span>
          <textarea
            className="input area"
            rows={2}
            spellCheck={false}
            placeholder="--width 1280 --height 720"
            value={settings.game_args.join("\n")}
            onChange={(e) =>
              save({
                game_args: e.target.value
                  .split("\n")
                  .map((line) => line.trim())
                  .filter(Boolean),
              })
            }
          />
        </label>
      </section>
    </div>
  );
}
