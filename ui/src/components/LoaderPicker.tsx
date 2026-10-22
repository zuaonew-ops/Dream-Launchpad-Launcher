import { actions, useLauncher } from "../store";
import type { LoaderKind } from "../types";
import Select from "./Select";

/** 加载器卡片：图标字母 + 名称 + 一句说明 */
const loaders: {
  value: LoaderKind;
  label: string;
  icon: string;
  desc: string;
  tint: string;
}[] = [
  {
    value: "none",
    label: "原版",
    icon: "MC",
    desc: "纯净游戏，不加任何加载器",
    tint: "var(--tint-vanilla)",
  },
  {
    value: "fabric",
    label: "Fabric",
    icon: "F",
    desc: "轻量、更新快，多数优化模组用它",
    tint: "var(--tint-fabric)",
  },
  {
    value: "quilt",
    label: "Quilt",
    icon: "Q",
    desc: "Fabric 的衍生版，兼容 Fabric 模组",
    tint: "var(--tint-quilt)",
  },
  {
    value: "forge",
    label: "Forge",
    icon: "F",
    desc: "老牌加载器，大型整合包常见",
    tint: "var(--tint-forge)",
  },
  {
    value: "neoforge",
    label: "NeoForge",
    icon: "N",
    desc: "Forge 分叉，1.20.2 之后的新选择",
    tint: "var(--tint-neoforge)",
  },
];

interface Props {
  mc: string;
  /** 底部快速安装条用紧凑版（只一行） */
  compact?: boolean;
}

/**
 * 装什么：加载器用卡片挑，版本用下拉。
 *
 * 之前是三个并排的原生下拉框，在 WebView 里弹出来的是系统列表，
 * 和界面完全不搭；改成卡片 + 自绘下拉。
 */
export default function LoaderPicker({ mc, compact }: Props) {
  const state = useLauncher();
  const loaderDisabled = state.loader === "none";
  const current = loaders.find((item) => item.value === state.loader) ?? loaders[0];

  if (compact) {
    return (
      <div className="picker-compact">
        <Select
          size="sm"
          value={state.loader}
          options={loaders.map((item) => ({ value: item.value, label: item.label }))}
          onChange={(value) => actions.setLoader(value as LoaderKind, mc)}
        />
        <Select
          size="sm"
          value={state.loaderVersion}
          placeholder={loaderDisabled ? "—" : "加载中…"}
          disabled={loaderDisabled || state.loaderOptions.length === 0}
          options={state.loaderOptions.map((c) => ({ value: c.version, label: c.label }))}
          onChange={(value) => actions.setLoaderVersion(value)}
        />
      </div>
    );
  }

  return (
    <div className="picker">
      <div className="loader-cards">
        {loaders.map((item) => (
          <button
            key={item.value}
            type="button"
            className={state.loader === item.value ? "loader-card active" : "loader-card"}
            onClick={() => actions.setLoader(item.value, mc)}
          >
            <span className="loader-icon" style={{ background: item.tint }}>
              {item.icon}
            </span>
            <span className="loader-text">
              <strong>{item.label}</strong>
              <small>{item.desc}</small>
            </span>
            {state.loader === item.value && (
              <svg className="loader-check" viewBox="0 0 16 16" width="14" height="14">
                <path
                  d="M3 8.5 6.5 12 13 4.5"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
            )}
          </button>
        ))}
      </div>

      <div className="picker-row">
        <label className="field">
          <span className="label">
            {current.label} 版本
            {state.loaderOptions.length > 0 && (
              <span className="value">{state.loaderOptions.length} 个可选</span>
            )}
          </span>
          <Select
            block
            value={state.loaderVersion}
            placeholder={
              loaderDisabled
                ? "原版不需要加载器版本"
                : state.loaderOptions.length === 0
                  ? "正在取版本列表…"
                  : "选一个版本"
            }
            disabled={loaderDisabled || state.loaderOptions.length === 0}
            options={state.loaderOptions.map((c) => ({
              value: c.version,
              label: c.label,
            }))}
            onChange={(value) => actions.setLoaderVersion(value)}
          />
        </label>

        <label className="field">
          <span className="label">
            高清修复 OptiFine
            <span className="label-actions">
              <button
                type="button"
                className={state.optifine ? "chip active" : "chip"}
                onClick={() => actions.setOptifine(mc, !state.optifine)}
              >
                {state.optifine ? "已加上" : "加上"}
              </button>
            </span>
          </span>
          <Select
            block
            value={state.optifineEdition}
            placeholder={state.optifine ? "选一个版本" : "不需要就留空"}
            disabled={!state.optifine || state.optifineOptions.length === 0}
            options={state.optifineOptions.map((c) => ({
              value: c.version,
              label: c.label,
            }))}
            onChange={(value) => actions.setOptifineEdition(value)}
          />
        </label>
      </div>
    </div>
  );
}
