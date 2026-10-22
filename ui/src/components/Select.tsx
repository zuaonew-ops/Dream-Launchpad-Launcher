import { useEffect, useRef, useState } from "react";

export interface SelectOption {
  value: string;
  label: string;
  /** 次要说明（右侧灰字） */
  hint?: string;
}

interface Props {
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  /** 宽度自适应容器 */
  block?: boolean;
  size?: "md" | "sm";
}

/**
 * 自绘下拉。原生 `<select>` 在 WebView 里弹的是系统窗口，样式跟不上，
 * 所以自己画一个：按钮 + 绝对定位的列表。
 */
export default function Select({
  value,
  options,
  onChange,
  placeholder = "请选择",
  disabled,
  block,
  size = "md",
}: Props) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const current = options.find((item) => item.value === value);

  // 点外面 / 按 Esc 关掉
  useEffect(() => {
    if (!open) return;
    const onDown = (event: MouseEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div
      ref={root}
      className={`sel${block ? " sel-block" : ""}${size === "sm" ? " sel-sm" : ""}${
        open ? " open" : ""
      }${disabled ? " disabled" : ""}`}
    >
      <button
        type="button"
        className="sel-btn"
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        <span className={current ? "sel-value" : "sel-value placeholder"}>
          {current?.label ?? placeholder}
        </span>
        <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true">
          <path
            d="M2.5 4.5 6 8l3.5-3.5"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </button>

      {open && options.length > 0 && (
        <ul className="sel-list" role="listbox">
          {options.map((option) => (
            <li key={option.value}>
              <button
                type="button"
                role="option"
                aria-selected={option.value === value}
                className={option.value === value ? "sel-item active" : "sel-item"}
                onClick={() => {
                  onChange(option.value);
                  setOpen(false);
                }}
              >
                <span>{option.label}</span>
                {option.hint && <span className="sel-hint">{option.hint}</span>}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
