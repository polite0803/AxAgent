// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";

export const Container: React.FC<DynamicUIProps> = ({
  schema,
  children,
}) => {
  const {
    padding,
    margin,
    display = "block",
    className,
  } = schema.props as {
    padding?: number | string;
    margin?: number | string;
    display?: "block" | "flex" | "inline-flex" | "inline-block" | "grid";
    className?: string;
  };

  const containerStyle: React.CSSProperties = {
    display,
    padding: typeof padding === "number" ? `${padding}px` : padding,
    margin: typeof margin === "number" ? `${margin}px` : margin,
    ...(schema.style as React.CSSProperties),
  };

  return (
    <div className={className} style={containerStyle}>
      {children}
    </div>
  );
};
