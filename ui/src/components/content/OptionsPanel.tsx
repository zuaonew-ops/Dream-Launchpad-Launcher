import { useEffect } from "react";
import { actions, useLauncher } from "../../store";
import Select from "../Select";
import type { GameOptions } from "../../types";

/** 游戏内设置：直接编辑 options.txt 里常见的几项 */
export default function OptionsPanel({ id }: { id: string }) {
  const state = useLauncher();
  const options = state.gameOptions;

  useEffect(() => {
    void actions.loadGameOptions(id);
  }, [id]);

  const set = (patch: Partial<GameOptions>) => {
    if (!options) return;
    void actions.saveGameOptions(id, { ...options, ...patch });
  };

  if (!options) {
    return (
      <section className="panel list-panel">
        <p className="hint">读取中…</p>
      </section>
    );
  }

  return (
    <section className="panel list-panel">
      <div className="content-tools">
        <span className="hint">
          {options.exists
            ? "直接改 options.txt；不认识的键会原样保留"
            : "还没进过游戏，改完会生成 options.txt"}
        </span>
      </div>

      <div className="grid">
        <section className="panel">
          <h2>画面</h2>
          <label className="field">
            <span className="label">渲染距离 {options.render_distance} 区块</span>
            <input
              type="range"
              min={2}
              max={32}
              value={options.render_distance}
              onChange={(e) => set({ render_distance: Number(e.target.value) })}
            />
          </label>
          <label className="field">
            <span className="label">模拟距离 {options.simulation_distance} 区块</span>
            <input
              type="range"
              min={2}
              max={32}
              value={options.simulation_distance}
              onChange={(e) => set({ simulation_distance: Number(e.target.value) })}
            />
          </label>
          <label className="field">
            <span className="label">最大帧率 {options.max_fps}</span>
            <input
              type="range"
              min={30}
              max={260}
              step={10}
              value={options.max_fps}
              onChange={(e) => set({ max_fps: Number(e.target.value) })}
            />
          </label>
          <label className="field">
            <span className="label">视场角 {options.fov}</span>
            <input
              type="range"
              min={30}
              max={110}
              value={options.fov}
              onChange={(e) => set({ fov: Number(e.target.value) })}
            />
          </label>
          <label className="field">
            <span className="label">亮度 {options.gamma.toFixed(1)}</span>
            <input
              type="range"
              min={0}
              max={1}
              step={0.1}
              value={options.gamma}
              onChange={(e) => set({ gamma: Number(e.target.value) })}
            />
          </label>
        </section>

        <section className="panel">
          <h2>其它</h2>
          <label className="field">
            <span className="label">画质</span>
            <Select
              block
              value={options.graphics}
              options={[
                { value: "fast", label: "流畅" },
                { value: "fancy", label: "精致" },
                { value: "fabulous", label: "极佳" },
              ]}
              onChange={(value) => set({ graphics: value })}
            />
          </label>
          <label className="field">
            <span className="label">粒子效果</span>
            <Select
              block
              value={options.particles}
              options={[
                { value: "all", label: "全部" },
                { value: "decreased", label: "减少" },
                { value: "minimal", label: "最少" },
              ]}
              onChange={(value) => set({ particles: value })}
            />
          </label>
          <label className="field">
            <span className="label">界面缩放（0 = 自动）</span>
            <Select
              block
              value={String(options.gui_scale)}
              options={[
                { value: "0", label: "自动" },
                { value: "1", label: "1×" },
                { value: "2", label: "2×" },
                { value: "3", label: "3×" },
                { value: "4", label: "4×" },
              ]}
              onChange={(value) => set({ gui_scale: Number(value) })}
            />
          </label>
          <label className="field">
            <span className="label">语言</span>
            <Select
              block
              value={options.language}
              options={[
                { value: "zh_cn", label: "简体中文" },
                { value: "zh_tw", label: "繁體中文" },
                { value: "en_us", label: "English" },
                { value: "ja_jp", label: "日本語" },
              ]}
              onChange={(value) => set({ language: value })}
            />
          </label>
          <label className="switch-row">
            <div>
              <span className="label">全屏启动</span>
              <p className="hint">改的是游戏里的全屏开关</p>
            </div>
            <input
              className="switch"
              type="checkbox"
              checked={options.fullscreen}
              onChange={(e) => set({ fullscreen: e.target.checked })}
            />
          </label>
        </section>
      </div>
    </section>
  );
}
