import { actions, useLauncher } from "../store";

/** 微软设备码登录面板：显示 8 位码、自动拉起浏览器、轮询状态。 */
export default function MsaLoginPanel() {
  const state = useLauncher();
  const msa = state.msa;
  if (!msa) return null;

  const code = msa.code;

  return (
    <div className="msa-panel">
      <div className="msa-head">
        <span className="msa-title">微软账号登录</span>
        <button type="button" className="icon-btn" title="关闭" onClick={actions.closeMsaLogin}>
          ×
        </button>
      </div>

      {msa.status === "requesting" && (
        <p className="msa-wait">
          <span className="spinner" />
          {msa.message}
        </p>
      )}

      {msa.status === "waiting" && code && (
        <>
          <p className="msa-hint">浏览器里应该已经打开了授权页，输入这串码：</p>
          <div className="msa-code">{code.user_code}</div>
          <div className="msa-actions">
            <button
              type="button"
              className="ghost"
              onClick={() => void actions.openUrl(code.verification_uri)}
            >
              重新打开浏览器
            </button>
            <span className="msa-wait">
              <span className="spinner" />
              {msa.message}
            </span>
          </div>
        </>
      )}

      {msa.status === "done" && <p className="msa-ok">{msa.message}</p>}
      {msa.status === "error" && <p className="msg error">{msa.message}</p>}
    </div>
  );
}
