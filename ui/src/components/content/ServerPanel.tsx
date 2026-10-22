import { useEffect, useState } from "react";
import { actions, useLauncher } from "../../store";

/** 服务器列表：直接读写游戏目录里的 servers.dat，进「多人游戏」就能看到 */
export default function ServerPanel({ id }: { id: string }) {
  const state = useLauncher();
  const [name, setName] = useState("");
  const [ip, setIp] = useState("");

  useEffect(() => {
    void actions.loadServers(id);
  }, [id]);

  const canAdd = name.trim() !== "" && ip.trim() !== "";

  return (
    <section className="panel list-panel">
      <div className="content-tools">
        <span className="hint">
          写进游戏目录的 servers.dat —— 启动游戏后在「多人游戏」里直接能看到
        </span>
        <div className="inline">
          <button
            type="button"
            className="ghost"
            onClick={() => void actions.pingServers(id)}
          >
            查在线状态
          </button>
        </div>
      </div>

      <div className="server-form">
        <label className="field">
          <span className="label">名称</span>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="朋友服"
          />
        </label>
        <label className="field">
          <span className="label">地址</span>
          <input
            value={ip}
            onChange={(e) => setIp(e.target.value)}
            placeholder="play.example.com:25565"
          />
        </label>
        <button
          type="button"
          className="pill primary"
          disabled={!canAdd}
          onClick={() => {
            void actions.addServer(id, name.trim(), ip.trim());
            setName("");
            setIp("");
          }}
        >
          添加
        </button>
      </div>

      {state.servers.length === 0 ? (
        <div className="empty-soft">
          <p className="hint">还没有服务器。</p>
        </div>
      ) : (
        <ul className="content-list">
          {state.servers.map((server) => {
            const status = state.serverStatus[server.ip];
            return (
            <li key={server.ip} className="content-item">
              <span className={status ? (status.online ? "dot on" : "dot") : "dot"} />
              <span className="content-name">{server.name}</span>
              <span className="content-meta">
                {server.ip}
                {status && (
                  <>
                    {" · "}
                    {status.online
                      ? `在线 ${status.players}/${status.max_players} · ${status.latency}ms`
                      : status.error}
                  </>
                )}
              </span>
              <span className="content-actions">
                <button
                  type="button"
                  className="ghost danger"
                  onClick={() => void actions.removeServer(id, server.ip)}
                >
                  删除
                </button>
              </span>
            </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
