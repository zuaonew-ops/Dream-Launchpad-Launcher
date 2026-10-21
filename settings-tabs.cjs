const fs = require("fs");
const p = "ui/src/pages/SettingsPage.tsx";
let s = fs.readFileSync(p, "utf8");

// 1) imports
s = s.replace(
  'import SavePanel from "../components/content/SavePanel";',
  `import SavePanel from "../components/content/SavePanel";
import PackPanel from "../components/content/PackPanel";
import OverviewPanel from "../components/settings/OverviewPanel";
import ModifyPanel from "../components/settings/ModifyPanel";`
);

// 2) tab 类型与条件
s = s.replace(
  'const [tab, setTab] = useState<"settings" | "mods" | "saves">("settings");',
  `type Tab = "overview" | "settings" | "modify" | "mods" | "saves" | "resourcepacks" | "shaderpacks";
  const [tab, setTab] = useState<Tab>("overview");`
);

// 3) 加载器判断（在 return 之前插入）
s = s.replace(
  "  const save = (patch: Partial<VersionSettings>) => {",
  `  // 这个版本的加载器决定导航条显示哪些页签：
  // 没有模组加载器就没有「模组」，没有光影加载器就没有「光影」
  const version = state.versions.find((item) => item.id === contentId);
  const loader = version?.loader ?? null;
  const hasModLoader = Boolean(loader) && loader !== "optifine";
  const hasShaderLoader = loader === "optifine";

  const tabs: { key: Tab; label: string; show: boolean }[] = [
    { key: "overview", label: "概况", show: true },
    { key: "settings", label: "设置", show: true },
    { key: "modify", label: "修改", show: true },
    { key: "mods", label: "模组", show: hasModLoader },
    { key: "saves", label: "存档", show: true },
    { key: "resourcepacks", label: "资源包", show: true },
    { key: "shaderpacks", label: "光影", show: hasModLoader || hasShaderLoader },
  ];

  const save = (patch: Partial<VersionSettings>) => {`
);

// 4) header：整块换掉
const headerStart = s.indexOf('      <header className="page-head">');
const headerEnd = s.indexOf("      </header>", headerStart);
if (headerStart < 0 || headerEnd < 0) {
  console.log("header not found");
  process.exit(1);
}
const newHeader = `      <header className="page-head">
        <div>
          <h1>版本设置</h1>
          <p className="desc">
            {contentId || "全局默认"}
            {loader ? ` · ${loader}` : " · 原版"}
            {" · "}
            {tabs.find((item) => item.key === tab)?.label}
          </p>
        </div>
        <div className="seg">
          {tabs
            .filter((item) => item.show)
            .map((item) => (
              <button
                key={item.key}
                type="button"
                className={tab === item.key ? "seg-item active" : "seg-item"}
                onClick={() => setTab(item.key)}
              >
                {item.label}
                {item.key === "mods" && state.mods.length > 0 && (
                  <span className="seg-count">{state.mods.length}</span>
                )}
                {item.key === "saves" && state.saves.length > 0 && (
                  <span className="seg-count">{state.saves.length}</span>
                )}
              </button>
            ))}
        </div>
`;
s = s.slice(0, headerStart) + newHeader + s.slice(headerEnd);

// 5) 内容分支：加 overview / modify / resourcepacks / shaderpacks
s = s.replace(
  `      {tab === "mods" && contentId ? (
        <ModPanel id={contentId} />
      ) : tab === "saves" && contentId ? (
        <SavePanel id={contentId} />
      ) : tab !== "settings" ? (`,
  `      {tab === "overview" && contentId ? (
        <OverviewPanel id={contentId} />
      ) : tab === "modify" && contentId ? (
        <ModifyPanel id={contentId} />
      ) : tab === "mods" && contentId ? (
        <ModPanel id={contentId} />
      ) : tab === "saves" && contentId ? (
        <SavePanel id={contentId} />
      ) : tab === "resourcepacks" && contentId ? (
        <PackPanel id={contentId} kind="resourcepacks" />
      ) : tab === "shaderpacks" && contentId ? (
        <PackPanel id={contentId} kind="shaderpacks" />
      ) : tab !== "settings" ? (`
);

// 6) 加载 mods/saves 的 effect 也覆盖新页签
s = s.replace(
  `    if (contentId && (tab === "mods" || tab === "saves")) void actions.loadContent(contentId);`,
  `    if (!contentId) return;
    if (tab === "mods" || tab === "saves" || tab === "overview") {
      void actions.loadContent(contentId);
    }
    if (tab === "overview" || tab === "resourcepacks") void actions.loadPacks(contentId, "resourcepacks");
    if (tab === "overview" || tab === "shaderpacks") void actions.loadPacks(contentId, "shaderpacks");`
);

fs.writeFileSync(p, s);
console.log("ok", s.includes("hasModLoader"), s.includes("OverviewPanel"), s.includes("PackPanel"));
