import { useEffect, useState, type ComponentType } from "react";
import DownloadPage from "./pages/DownloadPage";
import HomePage from "./pages/HomePage";
import SettingsPage from "./pages/SettingsPage";
import ToolsPage from "./pages/ToolsPage";
import { actions, useLauncher } from "./store";

type PageKey = "home" | "download" | "settings" | "tools";

const pages: Record<PageKey, ComponentType> = {
  home: HomePage,
  download: DownloadPage,
  settings: SettingsPage,
  tools: ToolsPage,
};

const nav: { key: PageKey; label: string }[] = [
  { key: "home", label: "主页" },
  { key: "download", label: "下载" },
  { key: "settings", label: "设置" },
  { key: "tools", label: "工具" },
];

function Icon({ name }: { name: PageKey }) {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
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
  const { environment } = useLauncher();
  const Current = pages[page];

  useEffect(() => {
    void actions.init();
  }, []);

  return (
    <div className="app">
      <nav className="rail">
        <div className="logo">DLP</div>

        <div className="nav">
          {nav.map((item) => (
            <button
              key={item.key}
              type="button"
              className={page === item.key ? "nav-item active" : "nav-item"}
              onClick={() => setPage(item.key)}
            >
              <Icon name={item.key} />
              <span>{item.label}</span>
            </button>
          ))}
        </div>

        <div className="rail-foot">{environment ? `v${environment.core}` : ""}</div>
      </nav>

      <main className="view">
        <div className="page-fade" key={page}>
          <Current />
        </div>
      </main>
    </div>
  );
}
