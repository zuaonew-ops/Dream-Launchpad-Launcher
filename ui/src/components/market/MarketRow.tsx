import { formatCount } from "./format";
import type { MarketHit } from "../../types";

interface Props {
  hit: MarketHit;
  onOpen: () => void;
  onInstall: () => void;
  busy: boolean;
}

/** 市场列表的一行：图标 + 中文名/原名 + 下载量 + 安装 */
export default function MarketRow({ hit, onOpen, onInstall, busy }: Props) {
  return (
    <li className="market-row">
      <button type="button" className="market-main" onClick={onOpen}>
        {hit.icon ? (
          <img className="market-icon" src={hit.icon} alt="" loading="lazy" />
        ) : (
          <span className="market-icon placeholder" />
        )}
        <span className="market-text">
          <span className="market-title">
            {hit.title_zh ?? hit.title}
            {hit.title_zh && <em>{hit.title}</em>}
          </span>
          <span className="market-desc">{hit.description}</span>
        </span>
      </button>

      <span className="market-downloads">{formatCount(hit.downloads)}</span>
      <button type="button" className="ghost" disabled={busy} onClick={onInstall}>
        安装
      </button>
    </li>
  );
}
