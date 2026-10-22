import { useEffect, useState } from "react";
import { actions, memoryCeiling, selectSelectedVersion, useLauncher } from "../store";
import ModPanel from "../components/content/ModPanel";
import SavePanel from "../components/content/SavePanel";
import ServerPanel from "../components/content/ServerPanel";
import SkinPanel from "../components/content/SkinPanel";
import PackPanel from "../components/content/PackPanel";
import OverviewPanel from "../components/settings/OverviewPanel";
import ModifyPanel from "../components/settings/ModifyPanel";
import type { VersionSettings } from "../types";

const FALLBACK: VersionSettings = {
  memory_mb: 2048,
  java_path: null,
  isolate: true,
  extra_jvm_args: [],
    game_args: [],
};

function formatMemory(mb: number): string {
  if (mb >= 1024) {
    const gb = mb / 1024;
    return `${Number.isInteger(gb) ? gb : gb.toFixed(1)} GB`;
  }
  return `${mb} MB`;
}

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
    | "resourcepacks"
    | "shaderpacks";
  const [tab, setTab] = useState<Tab>("settings");
  const installed = state.versions.filter((v) => v.installed);
  const selected = selectSelectedVersion(state);
  const key = state.settingsKey || selected?.id || "*";
  const settings = state.settings ?? FALLBACK;
  const ceiling = memoryCeiling(state.system?.total_mb || 16384);
  const memory = Math.min(settings.memory_mb ?? 2048, ceiling);

  const contentId = state.settingsKey || selected?.id || "";
  useEffect(() => {
    if (!contentId) return;
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
    { key: "resourcepacks", label: "资源包", show: !isGlobal },
    { key: "shaderpacks", label: "光影", show: !isGlobal && (hasModLoader || hasShaderLoader) },
  ];

  const save = (patch: Partial<VersionSettings>) => {
    void actions.saveSettings(key, { ...settings, ...patch });
  };

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
      ) : tab === "resourcepacks" && contentId ? (
        <PackPanel id={contentId} kind="resourcepacks" />
      ) : tab === "shaderpacks" && contentId ? (
        <PackPanel id={contentId} kind="shaderpacks" />
      ) : tab !== "settings" ? (
        <section className="panel empty-state">
          <h2>还没有选中版本</h2>
          <p className="hint">先去「下载」页装一个版本，模组和存档都挂在版本上</p>
        </section>
      ) : (
      <div className="grid">
        <section className="panel">
          <h2>账号与主题</h2>
          <p className="hint">账号在主页添加与切换；离线登录只用本地生成的 UUID（v3），不联网校验</p>

          <label className="switch-row">
            <div>
              <span className="label">浅色主题</span>
              <p className="hint">深色更护眼；切换有过渡动画</p>
            </div>
            <input
              className="switch"
              type="checkbox"
              checked={state.theme === "light"}
              onChange={(e) => actions.setTheme(e.target.checked ? "light" : "dark")}
            />
          </label>
        </section>

        <section className="panel">
          <h2>下载与市场</h2>
          <label className="field">
            <span className="label">下载源</span>
            <select
              className="input"
              value={state.appSettings?.download_source ?? "mirror"}
              onChange={(e) =>
                actions.setDownloadSource(e.target.value === "official" ? "official" : "mirror")
              }
            >
              <option value="mirror">镜像优先（BMCLAPI，国内快）</option>
              <option value="official">官方直连（镜像缺新版本时用）</option>
            </select>
          </label>
          <p className="hint">
            镜像只改写大文件地址；镜像拿不到时会自动回退官方
          </p>

          <label className="field">
            <span className="label">CurseForge API key</span>
            <input
              className="input"
              type="password"
              placeholder="不填就只能搜 Modrinth"
              value={state.appSettings?.curseforge_api_key ?? ""}
              onChange={(e) => actions.setCurseForgeKey(e.target.value)}
            />
          </label>
          <p className="hint">
            到 console.curseforge.com 免费申请；填了才能搜 CurseForge 和装 CurseForge 整合包
          </p>
        </section>

        <section className="panel">
          <h2>适用版本</h2>
          <label className="field">
            <span className="label">正在编辑</span>
            <select
              className="input"
              value={key}
              onChange={(e) => void actions.loadSettings(e.target.value)}
            >
              <option value="*">全局默认（所有版本的兜底值）</option>
              {installed.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.id}
                </option>
              ))}
            </select>
          </label>
          <p className="hint">
            {key === "*"
              ? "这里改的是兜底值；某个版本单独设过就以那个为准"
              : `下面的设置只对 ${key} 生效`}
          </p>
        </section>

        <section className="panel">
          <div className="section-head">
            <h2>本机</h2>
            <span className="hint">启动器能用多少资源</span>
          </div>
          <dl className="stat-grid">
            <div>
              <dt>物理内存</dt>
              <dd>{formatMemory(state.system?.total_mb ?? 0)}</dd>
            </div>
            <div>
              <dt>当前可用</dt>
              <dd>{formatMemory(state.system?.available_mb ?? 0)}</dd>
            </div>
            <div>
              <dt>CPU 核心</dt>
              <dd>{state.system?.cores ?? 0}</dd>
            </div>
            <div>
              <dt>已检测 Java</dt>
              <dd>{state.javas.length}</dd>
            </div>
          </dl>
        </section>
        <section className="panel">
          <h2>内存与 Java</h2>
          <label className="field">
            <span className="label">内存上限</span>
            <div className="inline">
              <input
                type="range"
                min={512}
                max={ceiling}
                step={256}
                value={memory}
                onChange={(e) => save({ memory_mb: Number(e.target.value) })}
              />
              <span className="value">{formatMemory(memory)}</span>
            </div>
            <div className="memory-meta">
              <span>
                系统共 {formatMemory(state.system?.total_mb ?? 0)}，当前可用{" "}
                {formatMemory(state.system?.available_mb ?? 0)}；上限按 80% 计算
              </span>
              <div className="memory-quick">
                {[2048, 4096, 8192, ceiling].map((mb, index) => (
                  <button
                    key={mb}
                    type="button"
                    className={memory === mb ? "chip active" : "chip"}
                    onClick={() => save({ memory_mb: mb })}
                  >
                    {index === 3 ? "建议" : formatMemory(mb)}
                  </button>
                ))}
              </div>
            </div>
          </label>

          <div className="field">
            <span className="label">下载 Java（Adoptium JRE，装到数据目录 runtime/）</span>
            <div className="inline">
              {[8, 17, 21, 25].map((major) => (
                <button
                  key={major}
                  type="button"
                  className="ghost"
                  disabled={state.busy}
                  onClick={() => void actions.installJava(major)}
                >
                  Java {major}
                </button>
              ))}
            </div>
          </div>

          <label className="field">
            <span className="label">Java 运行时</span>
            <select
              className="input"
              value={settings.java_path ?? ""}
              onChange={(e) => save({ java_path: e.target.value || null })}
            >
              <option value="">自动选择（按版本要求）</option>
              {state.javas.map((j) => (
                <option key={j.path} value={j.path}>
                  Java {j.major} · {j.version}
                </option>
              ))}
            </select>
          </label>
        </section>

        <section className="panel">
          <h2>启动</h2>
          <label className="switch-row">
            <div>
              <span className="label">版本隔离</span>
              <p className="hint">每个版本独立存档与 mods（versions/&lt;版本&gt;/）</p>
            </div>
            <input
              className="switch"
              type="checkbox"
              checked={settings.isolate ?? true}
              onChange={(e) => save({ isolate: e.target.checked })}
            />
          </label>

          <label className="field">
            <span className="label">额外 JVM 参数（一行一个）</span>
            <textarea
              className="input area"
              rows={3}
              spellCheck={false}
              placeholder="-XX:+UseG1GC"
              value={settings.extra_jvm_args.join("\n")}
              onChange={(e) =>
                save({
                  extra_jvm_args: e.target.value
                    .split("\n")
                    .map((line) => line.trim())
                    .filter(Boolean),
                })
              }
            />
          </label>

          <div className="field">
            <span className="label">数据目录</span>
            <code className="path">{state.environment?.data_dir ?? "…"}</code>
            <div className="inline">
              <button type="button" className="ghost" onClick={() => void actions.openDir("data")}>
                打开目录
              </button>
              <button type="button" className="ghost" onClick={() => void actions.openDir("logs")}>
                打开日志
              </button>
            </div>
          </div>
        </section>
      </div>
      )}
    </div>
  );
}
