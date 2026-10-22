import { useEffect, useState } from "react";
import { actions, selectSelectedVersion, useLauncher } from "../store";
import ModPanel from "../components/content/ModPanel";
import SavePanel from "../components/content/SavePanel";
import ServerPanel from "../components/content/ServerPanel";
import SkinPanel from "../components/content/SkinPanel";
import ShotPanel from "../components/content/ShotPanel";
import OptionsPanel from "../components/content/OptionsPanel";
import GlobalSettings from "../components/settings/GlobalSettings";
import VersionSettingsPanel from "../components/settings/VersionSettingsPanel";
import PackPanel from "../components/content/PackPanel";
import OverviewPanel from "../components/settings/OverviewPanel";
import ModifyPanel from "../components/settings/ModifyPanel";


export default function SettingsPage() {
  const state = useLauncher();
  type Tab =
    | "overview"
    | "settings"
    | "modify"
    | "mods"
    | "saves"
    | "servers"
    | "skins"
    | "shots"
    | "ingame"
    | "resourcepacks"
    | "shaderpacks";
  const [tab, setTab] = useState<Tab>("settings");
  const selected = selectSelectedVersion(state);
  const key = state.settingsKey || selected?.id || "*";

  const contentId = state.settingsKey || selected?.id || "";
  useEffect(() => {
    if (!contentId) return;
    if (tab === "ingame") {
      void actions.loadGameOptions(contentId);
    }
    if (tab === "shots") {
      void actions.loadShots(contentId);
    }
    if (tab === "skins") {
      void actions.loadSkins(contentId);
    }
    if (tab === "mods" || tab === "saves" || tab === "servers" || tab === "overview") {
      void actions.loadContent(contentId);
    }
    if (tab === "overview" || tab === "resourcepacks") {
      void actions.loadPacks(contentId, "resourcepacks");
    }
    if (tab === "overview" || tab === "shaderpacks") {
      void actions.loadPacks(contentId, "shaderpacks");
    }
  }, [contentId, tab]);

  // 这个版本的加载器决定导航条显示哪些页签：
  // 没有模组加载器就没有「模组」，没有光影加载器就没有「光影」
  const version = state.versions.find((item) => item.id === contentId);
  const loader = version?.loader ?? null;
  const hasModLoader = Boolean(loader) && loader !== "optifine";
  const hasShaderLoader = loader === "optifine";

  // 全局设置（侧边栏「设置」）只显示默认值；版本相关页签只在这个版本的页面里出现
  const isGlobal = key === "*";
  // 进版本页时默认停在「概况」
  useEffect(() => {
    if (!isGlobal) setTab("overview");
  }, [isGlobal, key]);

  const tabs: { key: Tab; label: string; show: boolean }[] = [
    { key: "overview", label: "概况", show: !isGlobal },
    { key: "settings", label: "设置", show: true },
    { key: "modify", label: "修改", show: !isGlobal },
    { key: "mods", label: "模组", show: !isGlobal && hasModLoader },
    { key: "saves", label: "存档", show: !isGlobal },
    { key: "servers", label: "服务器", show: !isGlobal },
    { key: "skins", label: "皮肤", show: !isGlobal && hasModLoader },
    { key: "shots", label: "截图", show: !isGlobal },
    { key: "ingame", label: "游戏内", show: !isGlobal },
    { key: "resourcepacks", label: "资源包", show: !isGlobal },
    { key: "shaderpacks", label: "光影", show: !isGlobal && (hasModLoader || hasShaderLoader) },
  ];

  return (
    <div className="page">
      <header className="page-head">
        <div>
          {!isGlobal && (
            <button
              type="button"
              className="ghost back"
              onClick={() => {
                actions.setDownloadTab("versions");
                window.dispatchEvent(new CustomEvent("dlp:nav", { detail: "download" }));
              }}
            >
              ← 返回下载
            </button>
          )}
          <h1>{isGlobal ? "设置" : "版本设置"}</h1>
          <p className="desc">
            {isGlobal
              ? "所有版本的默认值"
              : contentId + (loader ? " · " + loader : " · 原版")}
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
      </header>

      {tab === "overview" && contentId ? (
        <OverviewPanel id={contentId} />
      ) : tab === "modify" && contentId ? (
        <ModifyPanel id={contentId} />
      ) : tab === "mods" && contentId ? (
        <ModPanel id={contentId} />
      ) : tab === "saves" && contentId ? (
        <SavePanel id={contentId} />
      ) : tab === "servers" && contentId ? (
        <ServerPanel id={contentId} />
      ) : tab === "skins" && contentId ? (
        <SkinPanel id={contentId} />
      ) : tab === "shots" && contentId ? (
        <ShotPanel id={contentId} />
      ) : tab === "ingame" && contentId ? (
        <OptionsPanel id={contentId} />
      ) : tab === "resourcepacks" && contentId ? (
        <PackPanel id={contentId} kind="resourcepacks" />
      ) : tab === "shaderpacks" && contentId ? (
        <PackPanel id={contentId} kind="shaderpacks" />
      ) : tab !== "settings" ? (
        <section className="panel empty-state">
          <h2>还没有选中版本</h2>
          <p className="hint">先去「下载」页装一个版本，模组和存档都挂在版本上</p>
        </section>
      ) : isGlobal ? (
        <GlobalSettings />
      ) : (
        <VersionSettingsPanel id={contentId} />
      )}
    </div>
  );
}
