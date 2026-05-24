import { Check, ChevronsUpDown } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import { useTranslation } from "react-i18next";

interface SettingsSelectOption {
  label: ReactNode;
  value: string;
}

interface SettingsSelectProps {
  value?: string;
  onChange?: (value: string) => void;
  options: SettingsSelectOption[];
  style?: CSSProperties;
  disabled?: boolean;
  searchable?: boolean;
}

export function SettingsSelect({
  value,
  onChange,
  options,
  style,
  disabled,
  searchable,
}: SettingsSelectProps) {
  const { t } = useTranslation();
  const [hovered, setHovered] = useState(false);
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const containerRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  const currentLabel = options.find((o) => o.value === value)?.label ?? value;

  // click outside to close
  useEffect(() => {
    if (!open) { return; }
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
        setSearch("");
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const filteredOptions = useMemo(() => {
    if (!searchable || !search) { return options; }
    const q = search.toLowerCase();
    return options.filter((o) => {
      const text = typeof o.label === "string" ? o.label : o.value;
      return text.toLowerCase().includes(q);
    });
  }, [options, search, searchable]);

  const handleSelect = useCallback(
    (val: string) => {
      onChange?.(val);
      setOpen(false);
      setSearch("");
    },
    [onChange],
  );

  const trigger = (
    <div
      role="button"
      tabIndex={0}
      className="set-select-trigger"
      data-hovered={hovered || undefined}
      data-open={open || undefined}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{ ...style, cursor: disabled ? "not-allowed" : "pointer" }}
    >
      <span className="set-select-label">{currentLabel}</span>
      <ChevronsUpDown size={12} style={{ opacity: 0.4 }} />
    </div>
  );

  return (
    <div ref={containerRef} className="set-select">
      <div
        onClick={() => {
          if (!disabled) { setOpen(!open); }
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !disabled) { setOpen(!open); }
        }}
      >
        {trigger}
      </div>
      {open && (
        <div className="set-select-panel">
          {searchable && (
            <div className="set-select-search">
              <input
                ref={searchRef}
                className="set-input"
                placeholder={t("common.searchPlaceholder")}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                autoFocus
              />
            </div>
          )}
          <div className="set-select-options">
            {filteredOptions.map((opt) => (
              <button
                key={opt.value}
                className={`set-select-option${opt.value === value ? " active" : ""}`}
                onClick={() => handleSelect(opt.value)}
              >
                <span>{opt.label}</span>
                {opt.value === value && <Check size={14} />}
              </button>
            ))}
            {filteredOptions.length === 0 && <div className="set-select-empty">No results</div>}
          </div>
        </div>
      )}
    </div>
  );
}
