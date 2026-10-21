import { actions, useLauncher } from "../store";
import type { LoaderKind } from "../types";

const loaders: { value: LoaderKind; label: string }[] = [
  { value: "none", label: "不装加载器" },
  { value: "fabric", label: "Fabric" },
  { value: "quilt", label: "Quilt" },
  { value: "forge", label: "Forge" },
  { value: "neoforge", label: "NeoForge" },
];

/** 版本详情里的「加载器 / OptiFine」选择 + 安装入口。
 *  列表页和详情页都复用：列表页给的是底部快速安装条，详情页给的是完整面板。 */
export default function LoaderPicker({ mc, compact }: { mc: string; compact?: boolean }) {
  const state = useLauncher();
  const loaderDisabled = state.loader === "none";

  return (
    <div className={compact ? "options options-compact" : "options"}>
      <div className="option">
        <span className="label">模组加载器</span>
        <select
          className="input small"
          value={state.loader}
          onChange={(e) => actions.setLoader(e.target.value as LoaderKind, mc)}
        >
          {loaders.map((l) => (
            <option key={l.value} value={l.value}>
              {l.label}
            </option>
          ))}
        </select>
      </div>

      <div className="option">
        <span className="label">加载器版本</span>
        <select
          className="input small"
          value={state.loaderVersion}
          disabled={loaderDisabled || state.loaderOptions.length === 0}
          onChange={(e) => actions.setLoaderVersion(e.target.value)}
        >
          {state.loaderOptions.length === 0 && (
            <option value="">{loaderDisabled ? "—" : "加载中…"}</option>
          )}
          {state.loaderOptions.map((c) => (
            <option key={c.version} value={c.version}>
              {c.label}
            </option>
          ))}
        </select>
      </div>

      <label className="option check">
        <input
          type="checkbox"
          checked={state.optifine}
          onChange={(e) => actions.setOptifine(mc, e.target.checked)}
        />
        <span className="label">高清修复 OptiFine</span>
      </label>

      <div className="option">
        <span className="label">OptiFine 版本</span>
        <select
          className="input small"
          value={state.optifineEdition}
          disabled={!state.optifine || state.optifineOptions.length === 0}
          onChange={(e) => actions.setOptifineEdition(e.target.value)}
        >
          {state.optifineOptions.length === 0 && <option value="">—</option>}
          {state.optifineOptions.map((c) => (
            <option key={c.version} value={c.version}>
              {c.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
