import type { VersionSummary } from "../types";

interface Props {
  groups: { key: string; items: VersionSummary[] }[];
  selected: string | null;
  loading: boolean;
  onSelect: (id: string) => void;
}

export default function VersionList({ groups, selected, loading, onSelect }: Props) {
  if (loading) {
    return (
      <div className="list-wrap">
        <div className="skeletons">
          {Array.from({ length: 7 }, (_, i) => (
            <div key={i} className="skeleton" style={{ animationDelay: `${i * 60}ms` }} />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="list-wrap">
      {groups.map((group) => (
        <section className="group" key={group.key}>
          <header className="group-head">
            <span className="group-name">{group.key}</span>
            <span className="group-count">{group.items.length}</span>
          </header>

          <ul className="rows">
            {group.items.map((version) => (
              <li key={version.id}>
                <button
                  type="button"
                  className={version.id === selected ? "row active" : "row"}
                  onClick={() => onSelect(version.id)}
                >
                  <span className="bar" />
                  <span className="vid">{version.id}</span>
                  {version.local && <span className="kind local">本地</span>}
                  <span className={version.kind === "release" ? "kind" : "kind snapshot"}>
                    {version.kind === "release" ? "正式" : version.kind === "snapshot" ? "快照" : "其它"}
                  </span>
                  <span className={version.installed ? "state on" : "state"}>
                    {version.installed ? "已安装" : "未安装"}
                  </span>
                  <span className="time">{version.release_time.slice(0, 10) || "—"}</span>
                </button>
              </li>
            ))}
          </ul>
        </section>
      ))}

      {groups.length === 0 && <p className="empty">没有匹配的版本</p>}
    </div>
  );
}
