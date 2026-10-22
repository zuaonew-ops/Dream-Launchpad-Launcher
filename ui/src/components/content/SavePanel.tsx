import { actions, useLauncher } from "../../store";
import { formatBytes } from "../market/format";

/** 某个版本的存档列表 + 备份 */
export default function SavePanel({ id }: { id: string }) {
  const state = useLauncher();

  return (
    <section className="panel list-panel">
      <div className="content-tools">
        <span className="hint">每个世界一个文件夹，有 level.dat 才算有效；点「备份」打包成 zip</span>
        <div className="inline">
          <button
            type="button"
            className="ghost"
            onClick={() => void actions.openVersionDir(id, "saves")}
          >
            打开存档目录
          </button>
        </div>
      </div>

      {state.saves.length === 0 ? (
        <div className="empty-soft">
          <p className="hint">还没有存档。进游戏建一个世界，就会出现在这里。</p>
        </div>
      ) : (
        <ul className="content-list">
          {state.saves.map((save, index) => (
            <li
              key={save.name}
              className="content-item"
              style={{ animationDelay: `${Math.min(index, 12) * 25}ms` }}
            >
              <span className={save.valid ? "dot on" : "dot"} />
              <span className="content-name">{save.name}</span>
              <span className="content-meta">{save.modified}</span>
              <span className="content-actions">
                <button
                  type="button"
                  className="ghost"
                  disabled={!save.valid}
                  onClick={() => void actions.loadLevelInfo(id, save.name)}
                >
                  {state.levelInfo[save.name] ? "收起" : "详情"}
                </button>
                <button
                  type="button"
                  className="ghost"
                  disabled={!save.valid}
                  onClick={() => void actions.backupSave(id, save.name)}
                >
                  备份
                </button>
              </span>
              {state.levelInfo[save.name] && (
                <div className="save-detail">
                  {(() => {
                    const info = state.levelInfo[save.name]!;
                    const mode =
                      ["生存", "创造", "冒险", "旁观"][info.game_type] ?? "未知";
                    const diff =
                      ["未知", "和平", "简单", "普通", "困难"][info.difficulty] ?? "未知";
                    return (
                      <>
                        <span>种子 <code>{info.seed}</code></span>
                        <span>模式 {mode}{info.hardcore ? "（极限）" : ""}</span>
                        <span>难度 {diff}</span>
                        {info.version && <span>存档版本 {info.version}</span>}
                        {info.name && info.name !== save.name && <span>世界名 {info.name}</span>}
                      </>
                    );
                  })()}
                </div>
              )}
            </li>
          ))}
        </ul>
      )}

      <div className="section-head">
        <h2>备份</h2>
        <span className="hint">放在游戏目录的 dlp-backups/ 下，还原会覆盖同名世界</span>
      </div>

      {state.backups.length === 0 ? (
        <div className="empty-soft">
          <p className="hint">还没有备份。</p>
        </div>
      ) : (
        <ul className="content-list">
          {state.backups.map((backup) => (
            <li key={backup.name} className="content-item">
              <span className="dot on" />
              <span className="content-name">{backup.world}</span>
              <span className="content-meta">
                {backup.time} · {formatBytes(backup.bytes)}
              </span>
              <span className="content-actions">
                <button
                  type="button"
                  className="ghost"
                  onClick={() => void actions.restoreBackup(id, backup.name)}
                >
                  还原
                </button>
                <button
                  type="button"
                  className="ghost danger"
                  onClick={() => void actions.deleteBackup(id, backup.name)}
                >
                  删除
                </button>
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
