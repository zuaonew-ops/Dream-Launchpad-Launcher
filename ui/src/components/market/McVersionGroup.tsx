import { actions, useLauncher } from "../../store";
import type { MarketVersion } from "../../types";
import { formatBytes, formatCount, formatDate } from "./format";

interface Props {
  /** 这个分组对应的《我的世界》版本号 */
  mc: string;
  versions: MarketVersion[];
  /** 当前选中的游戏版本与加载器，用来判断「最适配」 */
  currentMc: string;
  currentLoader: string;
  expanded: string | null;
  onToggle: (key: string) => void;
}

/** 按《我的世界》版本分组：一个版本一行，点开看它下面挂的文件 */
export default function McVersionGroup({
  mc,
  versions,
  currentMc,
  currentLoader,
  expanded,
  onToggle,
}: Props) {
  const state = useLauncher();
  const key = `mc:${mc}`;
  const open = expanded === key;
  const best = mc === currentMc;

  // 组内先把文件摊平：一个模组版本可能同时挂 fabric / neoforge 两个文件
  const files = versions.flatMap((version) =>
    version.files.map((file) => ({
      file,
      loaders: version.loaders,
      versionNumber: version.version_number || version.name,
      date: version.date,
      downloads: version.downloads,
    }))
  );
  const loaderHit = files.some(
    (item) => currentLoader === "any" || item.loaders.includes(currentLoader)
  );

  return (
    <li className={best && loaderHit ? "version-row best" : "version-row"}>
      <button type="button" className="version-head" onClick={() => onToggle(key)}>
        <span className="version-name">{mc}</span>
        {best && loaderHit && <span className="tag-best">最适配</span>}
        <span className="version-meta">{files.length} 个文件</span>
        <span className="version-date">{formatDate(versions[0]?.date ?? "")}</span>
        <span className="version-downloads">{formatCount(versions[0]?.downloads ?? 0)}</span>
        <span className={open ? "chev open" : "chev"} />
      </button>

      {open && (
        <ul className="file-list">
          {files.map((item, index) => (
            <li className="file-row" key={`${item.file.version_id}-${item.file.filename}-${index}`}>
              <span className="file-loaders">
                {item.loaders.length > 0 ? item.loaders.join(" / ") : "通用"}
              </span>
              <span className="file-name" title={item.file.filename}>
                {item.file.filename}
              </span>
              <span className="file-version">{item.versionNumber}</span>
              <span className="file-size">{formatBytes(item.file.size)}</span>
              <button
                type="button"
                className="ghost"
                disabled={state.busy}
                onClick={() => actions.installMarketFile(item.file.download_url, item.file.filename)}
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
