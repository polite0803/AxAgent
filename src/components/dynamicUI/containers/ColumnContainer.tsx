// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";

export const ColumnContainer: React.FC<DynamicUIProps> = ({
  schema,
  children,
}) => {
  const { gap = 8, align = "stretch", className } = schema.props as {
    gap?: number;
    align?: string;
    className?: string;
  };

  return (
    <div
      className={`flex flex-col ${className || ""}`}
      style={{
        gap: `${gap}px`,
        alignItems: align,
        ...(schema.style as React.CSSProperties),
      }}
    >
      {children}
    </div>
  );
};
