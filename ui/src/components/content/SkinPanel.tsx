import { useEffect, useState } from "react";
import { actions, useLauncher } from "../../store";
import { formatBytes } from "../market/format";

/** 皮肤：存一组 PNG，按玩家名放进 CustomSkinLoader 的本地皮肤目录 */
export default function SkinPanel({ id }: { id: string }) {
  const state = useLauncher();
  const [player, setPlayer] = useState("");

  useEffect(() => {
    void actions.loadSkins(id);
  }, [id]);

  return (
    <section className="panel list-panel">
      <div className="content-tools">
        <span className="hint">
          皮肤文件放在游戏目录的 dlp-skins/；装 CustomSkinLoader 模组后按玩家名生效
        </span>
        <div className="inline">
          <button
            type="button"
            className="pill"
            onClick={() => void actions.importSkin(id)}
          >
            导入皮肤…
          </button>
        </div>
      </div>

      {!state.hasCsl && (
        <p className="hint warn">
          这个版本没装 CustomSkinLoader —— 皮肤能存进来，但游戏里不会显示。
          先去「资源」页搜 customskinloader 装上。
        </p>
      )}

      <div className="server-form">
        <label className="field">
          <span className="label">游戏内玩家名</span>
          <input
            value={player}
            onChange={(e) => setPlayer(e.target.value)}
            placeholder="Steve"
          />
        </label>
      </div>

      {state.skins.length === 0 ? (
        <div className="empty-soft">
          <p className="hint">皮肤库还是空的。64×64 或 64×32 的 PNG 都行。</p>
        </div>
      ) : (
        <ul className="content-list">
          {state.skins.map((skin) => (
            <li key={skin.name} className="content-item">
              <span className="dot on" />
              <span className="content-name">{skin.name}</span>
              <span className="content-meta">
                {skin.width}×{skin.height} · {formatBytes(skin.bytes)}
              </span>
              <span className="content-actions">
                <button
                  type="button"
                  className="ghost"
                  disabled={!state.hasCsl || player.trim() === ""}
                  onClick={() => void actions.applySkin(id, skin.name, player.trim())}
                >
                  装给这个玩家
                </button>
                <button
                  type="button"
                  className="ghost danger"
                  onClick={() => void actions.deleteSkin(id, skin.name)}
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
