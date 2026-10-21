interface Props {
  title: string;
  message: string;
  /** 确认按钮文案 */
  confirmText?: string;
  danger?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

/**
 * 应用内确认框。
 *
 * 不用 `window.confirm` —— Tauri 的 WebView2 会拦掉原生对话框，
 * 调用直接返回 false，按钮看起来就像「点了没反应」。
 */
export default function ConfirmDialog({
  title,
  message,
  confirmText = "确认",
  danger = false,
  onCancel,
  onConfirm,
}: Props) {
  return (
    <div className="launch-overlay" role="dialog" aria-modal="true">
      <div className="launch-card confirm-card">
        <div className="launch-head">
          <span className={danger ? "launch-badge error" : "launch-badge"}>{danger ? "不可恢复" : "确认"}</span>
          <strong>{title}</strong>
        </div>
        <p className="confirm-message">{message}</p>
        <div className="install-actions">
          <button type="button" className="ghost" onClick={onCancel}>
            取消
          </button>
          <button type="button" className={danger ? "danger" : "primary"} onClick={onConfirm}>
            {confirmText}
          </button>
        </div>
      </div>
    </div>
  );
}
