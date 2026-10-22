import { useEffect } from "react";
import { actions, useLauncher } from "../../store";
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
            <select
              className="input"
              value={options.graphics}
              onChange={(e) => set({ graphics: e.target.value })}
            >
              <option value="fast">流畅</option>
              <option value="fancy">精致</option>
              <option value="fabulous">极佳</option>
            </select>
          </label>
          <label className="field">
            <span className="label">粒子效果</span>
            <select
              className="input"
              value={options.particles}
              onChange={(e) => set({ particles: e.target.value })}
            >
              <option value="all">全部</option>
              <option value="decreased">减少</option>
              <option value="minimal">最少</option>
            </select>
          </label>
          <label className="field">
            <span className="label">界面缩放（0 = 自动）</span>
            <select
              className="input"
              value={options.gui_scale}
              onChange={(e) => set({ gui_scale: Number(e.target.value) })}
            >
              <option value={0}>自动</option>
              <option value={1}>1×</option>
              <option value={2}>2×</option>
              <option value={3}>3×</option>
              <option value={4}>4×</option>
            </select>
          </label>
          <label className="field">
            <span className="label">语言</span>
            <select
              className="input"
              value={options.language}
              onChange={(e) => set({ language: e.target.value })}
            >
              <option value="zh_cn">简体中文</option>
              <option value="zh_tw">繁體中文</option>
              <option value="en_us">English</option>
              <option value="ja_jp">日本語</option>
            </select>
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
