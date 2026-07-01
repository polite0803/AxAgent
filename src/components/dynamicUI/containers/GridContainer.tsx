// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";

export const GridContainer: React.FC<DynamicUIProps> = ({
  schema,
  children,
}) => {
  const { columns = 2, gap = 16, className } = schema.props as {
    columns?: number;
    gap?: number;
    className?: string;
  };

  return (
    <div
      className={`grid ${className || ""}`}
      style={{
        gridTemplateColumns: `repeat(${columns}, 1fr)`,
        gap: `${gap}px`,
        ...(schema.style as React.CSSProperties),
      }}
    >
      {children}
    </div>
  );
};
