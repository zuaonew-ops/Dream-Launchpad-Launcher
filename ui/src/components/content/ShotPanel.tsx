import { useEffect } from "react";
import { actions, useLauncher } from "../../store";
import { formatBytes } from "../market/format";

/** 游戏截图：列表、用系统看图打开、删除 */
export default function ShotPanel({ id }: { id: string }) {
  const state = useLauncher();

  useEffect(() => {
    void actions.loadShots(id);
  }, [id]);

  return (
    <section className="panel list-panel">
      <div className="content-tools">
        <span className="hint">游戏里按 F2 截的图都在这里（screenshots/）</span>
        <div className="inline">
          <button
            type="button"
            className="ghost"
            onClick={() => void actions.openVersionDir(id, "instance")}
          >
            打开版本目录
          </button>
        </div>
      </div>

      {state.shots.length === 0 ? (
        <div className="empty-soft">
          <p className="hint">还没有截图。</p>
        </div>
      ) : (
        <ul className="content-list">
          {state.shots.map((shot) => (
            <li key={shot.name} className="content-item">
              <span className="dot on" />
              <span className="content-name">{shot.name}</span>
              <span className="content-meta">
                {shot.width > 0 ? `${shot.width}×${shot.height} · ` : ""}
                {formatBytes(shot.bytes)} · {shot.time}
              </span>
              <span className="content-actions">
                <button
                  type="button"
                  className="ghost"
                  onClick={() => void actions.openShot(id, shot.name)}
                >
                  打开
                </button>
                <button
                  type="button"
                  className="ghost danger"
                  onClick={() => void actions.deleteShot(id, shot.name)}
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
