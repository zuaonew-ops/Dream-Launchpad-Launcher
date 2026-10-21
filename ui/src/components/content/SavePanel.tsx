import { actions, useLauncher } from "../../store";

/** 某个版本的存档列表 */
export default function SavePanel({ id }: { id: string }) {
  const state = useLauncher();

  return (
    <section className="panel list-panel">
      <div className="content-tools">
        <span className="hint">每个世界一个文件夹，有 level.dat 才算有效</span>
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
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
