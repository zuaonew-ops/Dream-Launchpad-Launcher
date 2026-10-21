import { actions, useLauncher } from "../../store";

/** 「修改」：修复、删版本、装加载器这类会动文件的操作 */
export default function ModifyPanel({ id }: { id: string }) {
  const state = useLauncher();
  const version = state.versions.find((item) => item.id === id);

  return (
    <div className="grid">
      <section className="panel">
        <h2>文件校验</h2>
        <p className="hint">
          对照官方清单逐个校验已下载的文件，缺什么补什么。存档和模组不会被改动。
        </p>
        <div className="inline">
          <button
            type="button"
            className="primary"
            disabled={state.busy}
            onClick={() => void actions.repairVersion(id)}
          >
            {state.busy ? "校验中…" : "校验修复"}
          </button>
          <button
            type="button"
            className="ghost"
            onClick={() => void actions.openVersionDir(id, "instance")}
          >
            打开版本目录
          </button>
        </div>
      </section>

      <section className="panel">
        <h2>加载器</h2>
        <p className="hint">
          {version?.loader
            ? `当前：${version.loader}。想换加载器就去「下载」页重新装一个版本，不同加载器不覆盖安装。`
            : "当前是原版。需要模组就去「下载」页选 Fabric / Forge / NeoForge 重新装一个。"}
        </p>
        <div className="inline">
          <button
            type="button"
            className="ghost"
            onClick={() => {
              actions.setDownloadTab("versions");
              window.dispatchEvent(new CustomEvent("dlp:nav", { detail: "download" }));
            }}
          >
            去下载页
          </button>
        </div>
      </section>

      <section className="panel">
        <h2>删除</h2>
        <p className="hint">删掉这个版本的安装文件。存档和配置默认保留，除非你勾了隔离目录。</p>
        <button
          type="button"
          className="danger"
          disabled={state.busy}
          onClick={() => {
            if (!confirm(`删除版本 ${id}？这个操作不可恢复。`)) return;
            void actions.deleteVersion(id);
          }}
        >
          删除这个版本
        </button>
      </section>
    </div>
  );
}
