// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";

export const RowContainer: React.FC<DynamicUIProps> = ({
  schema,
  children,
}) => {
  const { gap = 8, align = "center", justify = "start", wrap, className } = schema.props as {
    gap?: number;
    align?: string;
    justify?: string;
    wrap?: boolean;
    className?: string;
  };

  return (
    <div
      className={`flex flex-row ${wrap ? "flex-wrap" : ""} ${className || ""}`}
      style={{
        gap: `${gap}px`,
        alignItems: align,
        justifyContent: justify,
        ...(schema.style as React.CSSProperties),
      }}
    >
      {children}
    </div>
  );
};
