import { actions, useLauncher } from "../../store";
import type { MarketVersion } from "../../types";
import { formatBytes, formatCount, formatDate } from "./format";

interface Props {
  version: MarketVersion;
  /** 当前选中的游戏版本与加载器，用来判断「最适配」 */
  mc: string;
  loader: string;
  expanded: boolean;
  onToggle: () => void;
}

/** 一个版本：一行摘要，点开显示它挂的文件 */
export default function VersionFiles({ version, mc, loader, expanded, onToggle }: Props) {
  const state = useLauncher();
  const versionMatches = !mc || version.game_versions.includes(mc);
  const loaderMatches =
    loader === "any" ||
    loader === "vanilla" && version.loaders.length === 0 ||
    version.loaders.includes(loader);
  const best = versionMatches && loaderMatches;

  return (
    <li className={best ? "version-row best" : "version-row"}>
      <button type="button" className="version-head" onClick={onToggle}>
        <span className="version-name">{version.version_number || version.name}</span>
        {best && <span className="tag-best">最适配</span>}
        <span className="version-meta">
          {version.game_versions.slice(0, 3).join(" / ") || "不限版本"}
          {version.loaders.length > 0 && ` · ${version.loaders.join(" / ")}`}
        </span>
        <span className="version-date">{formatDate(version.date)}</span>
        <span className="version-downloads">{formatCount(version.downloads)}</span>
        <span className={expanded ? "chev open" : "chev"} />
      </button>

      {expanded && (
        <ul className="file-list">
          {version.files.length === 0 && <li className="hint">这一版没有可下载文件</li>}
          {version.files.map((file) => (
            <li className="file-row" key={`${file.version_id}-${file.filename}`}>
              <span className="file-name" title={file.filename}>
                {file.filename}
              </span>
              <span className="file-size">{formatBytes(file.size)}</span>
              <button
                type="button"
                className="ghost"
                disabled={state.busy || !state.selected}
                title={state.selected ? "下载到该版本的 mods 目录" : "先在下载页选一个版本"}
                onClick={() => void actions.installMarketFile(file.download_url, file.filename)}
              >
                下载
              </button>
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}
