import { useEffect } from "react";
import { actions, useLauncher } from "../../store";

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return bytes > 0 ? `${bytes} B` : "文件夹";
}

interface Props {
  id: string;
  kind: "resourcepacks" | "shaderpacks";
}

/** 资源包 / 光影：列目录里的 zip 与文件夹 */
export default function PackPanel({ id, kind }: Props) {
  const state = useLauncher();
  const list = state.packs[kind];
  const label = kind === "resourcepacks" ? "资源包" : "光影";

  useEffect(() => {
    void actions.loadPacks(id, kind);
  }, [id, kind]);

  return (
    <section className="panel list-panel">
      <div className="content-tools">
        <span className="hint">
          {label}放到 <code>{kind}/</code> 目录，zip 和文件夹都认
        </span>
        <button
          type="button"
          className="ghost"
          onClick={() => void actions.openVersionDir(id, kind)}
        >
          打开 {kind} 目录
        </button>
      </div>

      {list.length === 0 ? (
        <div className="empty-soft">
          <div className="empty-icon">◍</div>
          <p className="hint">
            还没有{label}。到「下载 → 资源」里选「{label === "光影" ? "光影" : "材质"}」装上，或者手动放进目录。
          </p>
        </div>
      ) : (
        <ul className="content-list">
          {list.map((item, index) => (
            <li
              key={item.name}
              className="content-item"
              style={{ animationDelay: `${Math.min(index, 12) * 25}ms` }}
            >
              <span className={item.jar ? "dot on" : "dot"} />
              <span className="content-name" title={item.name}>
                {item.name}
              </span>
              <span className="content-meta">{formatSize(item.size)}</span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
