import type { TypeFilter } from "../store";

const tabs: { key: TypeFilter; label: string }[] = [
  { key: "release", label: "正式版" },
  { key: "snapshot", label: "快照" },
  { key: "installed", label: "已安装" },
  { key: "local", label: "本地" },
  { key: "all", label: "全部" },
];

interface Props {
  keyword: string;
  filter: TypeFilter;
  loading: boolean;
  total: number;
  onKeyword: (value: string) => void;
  onFilter: (value: TypeFilter) => void;
  onRefresh: () => void;
}

export default function VersionToolbar({
  keyword,
  filter,
  loading,
  total,
  onKeyword,
  onFilter,
  onRefresh,
}: Props) {
  return (
    <div className="toolbar">
      <div className="tabs">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            type="button"
            className={filter === tab.key ? "tab active" : "tab"}
            onClick={() => onFilter(tab.key)}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <label className="search">
        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
          <path
            d="M10 4a6 6 0 1 0 3.8 10.6l4.3 4.3 1.4-1.4-4.3-4.3A6 6 0 0 0 10 4zm0 2a4 4 0 1 1 0 8 4 4 0 0 1 0-8z"
            fill="currentColor"
          />
        </svg>
        <input
          value={keyword}
          placeholder="搜索版本号"
          spellCheck={false}
          onChange={(e) => onKeyword(e.target.value)}
        />
        <span className="count">{total}</span>
      </label>

      <button type="button" className="ghost" disabled={loading} onClick={onRefresh}>
        {loading ? "刷新中" : "刷新"}
      </button>
    </div>
  );
}
