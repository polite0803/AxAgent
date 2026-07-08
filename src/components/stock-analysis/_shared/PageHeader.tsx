import { type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

interface PageHeaderProps {
  /** i18n 键，例如 "watchlist.title" */
  titleKey: string;
  /** 点击返回按钮时跳转的路径，默认 "/" */
  backTo?: string;
  /** 返回按钮的 i18n 键，默认 "nav.chat" */
  backLabelKey?: string;
  /** 标题右侧的元信息文本/节点 */
  meta?: ReactNode;
  /** 标题最右侧的按钮组 */
  actions?: ReactNode;
}

/**
 * 5 个新页面的统一头部：返回按钮 + 标题 + 右侧 meta/actions。
 * 复用 sa-header / sa-header-back / sa-header-title / sa-header-meta 样式。
 */
export function PageHeader({
  titleKey,
  backTo = "/",
  backLabelKey = "nav.chat",
  meta,
  actions,
}: PageHeaderProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  return (
    <header className="sa-header">
      <button type="button" className="sa-header-back" onClick={() => navigate(backTo)}>
        ‹ {t(backLabelKey)}
      </button>
      <h2 className="sa-header-title">{t(titleKey)}</h2>
      {meta && <span className="sa-header-meta">{meta}</span>}
      {actions && <div className="ml-auto flex items-center gap-2">{actions}</div>}
    </header>
  );
}
