import { useEffect, useState, type ComponentType } from "react";
import DownloadPage from "./pages/DownloadPage";
import HomePage from "./pages/HomePage";
import SettingsPage from "./pages/SettingsPage";
import TaskPage from "./pages/TaskPage";
import ToolsPage from "./pages/ToolsPage";
import LaunchOverlay from "./components/LaunchOverlay";
import InstallDialog from "./components/market/InstallDialog";
import { actions, useLauncher } from "./store";

type PageKey = "home" | "download" | "task" | "settings" | "version" | "tools";

const pages: Record<PageKey, ComponentType> = {
  home: HomePage,
  download: DownloadPage,
  task: TaskPage,
  settings: SettingsPage,
  version: SettingsPage,
  tools: ToolsPage,
};

const navGroups: { label: string; items: { key: PageKey; label: string }[] }[] = [
  {
    label: "游戏",
    items: [
      { key: "home", label: "主页" },
      { key: "download", label: "下载" },
    ],
  },
  {
    label: "其它",
    items: [
      { key: "settings", label: "设置" },
      { key: "tools", label: "工具" },
    ],
  },
];

function Icon({ name }: { name: PageKey }) {
  return (
    <svg viewBox="0 0 24 24" width="17" height="17" aria-hidden="true">
      {name === "home" && <path d="M4 11 12 4l8 7v9h-6v-6h-4v6H4z" />}
      {name === "download" && <path d="M12 4v11m0 0 4-4m-4 4-4-4M5 19h14" />}
      {name === "settings" && (
        <>
          <path d="M5 8h14M5 16h14" />
          <circle cx="9" cy="8" r="2" />
          <circle cx="15" cy="16" r="2" />
        </>
      )}
      {name === "tools" && <path d="M12 3 3 7.5v9L12 21l9-4.5v-9L12 3zM3 7.5 12 12l9-4.5M12 12v9" />}
    </svg>
  );
}

export default function App() {
  const [page, setPage] = useState<PageKey>("home");
  const state = useLauncher();
  const Current = pages[page];
  const dark = state.theme === "dark";
  const job = state.install;

  useEffect(() => {
    void actions.init();
    // 启动几秒后再查有无新版本，别和首屏加载抢带宽
    const timer = window.setTimeout(() => void actions.checkLauncherUpdate(), 3000);
    return () => window.clearTimeout(timer);
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = state.theme;
  }, [state.theme]);

  useEffect(() => {
    const onNav = (e: Event) => setPage((e as CustomEvent<PageKey>).detail);
    window.addEventListener("dlp:nav", onNav);
    return () => window.removeEventListener("dlp:nav", onNav);
  }, []);

  return (
    <div className="app">
      <nav className="rail">
        <div className="brand">
          <div className="brand-mark">DLP</div>
          <div className="brand-text">
            <span className="brand-name">Dream Launchpad</span>
            <span className="brand-sub">Launcher</span>
          </div>
        </div>

        {navGroups.map((group) => (
          <div className="nav-group" key={group.label}>
            <span className="nav-group-label">{group.label}</span>
            {group.items.map((item) => (
              <button
                key={item.key}
                type="button"
                className={page === item.key ? "nav-item active" : "nav-item"}
                onClick={() => {
                  if (item.key === "settings") actions.openGlobalSettings();
                  setPage(item.key);
                }}
              >
                <Icon name={item.key} />
                <span>{item.label}</span>
              </button>
            ))}
          </div>
        ))}

        <div className="rail-foot">
          <span>{state.environment ? `v${state.environment.core}` : ""}</span>
          <button
            type="button"
            className="theme-btn"
            title={dark ? "切换到浅色" : "切换到深色"}
            onClick={() => actions.setTheme(dark ? "light" : "dark")}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
              <g fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round">
                {dark ? (
                  <>
                    <circle cx="12" cy="12" r="4" />
                    <path d="M12 3v2m0 14v2M3 12h2m14 0h2M5.6 5.6l1.4 1.4m10 10 1.4 1.4m0-12.8-1.4 1.4m-10 10L5.6 18.4" />
                  </>
                ) : (
                  <path d="M20 14.5A8.5 8.5 0 1 1 9.5 4a7 7 0 0 0 10.5 10.5z" />
                )}
              </g>
            </svg>
          </button>
        </div>
      </nav>

      <main className="view">
        <div className="page-fade" key={page}>
          <Current />
        </div>
      </main>

      <LaunchOverlay />
      <InstallDialog />

      {/* 全局任务条：下载 / 安装 / 修复都在这里看，和当前在哪个页面无关 */}
      {job && (
        <div className="job-bar">
          <span className="job-dot" />
          <div className="job-text">
            <strong>{job.id}</strong>
            <span>{job.stage}</span>
          </div>
          <div className="job-meter">
            <span style={{ width: `${Math.round(job.percent * 100)}%` }} />
          </div>
          <span className="job-pct">{Math.round(job.percent * 100)}%</span>
          <button type="button" className="ghost small" onClick={actions.goTaskPage}>
            下载详情
          </button>
        </div>
      )}
    </div>
  );
}
