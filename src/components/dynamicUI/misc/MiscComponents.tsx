// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Button, Divider, Image, Progress, Tag, Typography } from "antd";

const { Text, Title } = Typography;

/**
 * 按钮组件，基于 Ant Design Button。
 */
export const DynamicButton: React.FC<DynamicUIProps> = ({
  schema,
  onAction,
}) => {
  const {
    text = "按钮",
    type = "default",
    size = "middle",
    disabled = false,
    danger = false,
    loading = false,
    block = false,
    ...rest
  } = schema.props as {
    text?: string;
    type?: "primary" | "default" | "dashed" | "link" | "text";
    size?: "small" | "middle" | "large";
    disabled?: boolean;
    danger?: boolean;
    loading?: boolean;
    block?: boolean;
    icon?: string;
    [key: string]: unknown;
  };

  const handleClick = () => {
    const clickHandler = schema.events?.find((e) => e.trigger === "onClick");
    if (clickHandler && onAction) {
      for (const action of clickHandler.actions) {
        onAction(action);
      }
    }
  };

  return (
    <Button
      type={type}
      size={size}
      disabled={disabled}
      danger={danger}
      loading={loading}
      block={block}
      onClick={handleClick}
      style={schema.style as React.CSSProperties}
      {...rest}
    >
      {text}
    </Button>
  );
};

/**
 * 文本组件，基于 Ant Design Typography.Text / Title。
 */
export const DynamicText: React.FC<DynamicUIProps> = ({ schema }) => {
  const {
    content = "",
    level,
    type,
    strong,
    italic,
    code,
    delete: deleteStyle,
    copyable,
  } = schema.props as {
    content?: string;
    level?: 1 | 2 | 3 | 4 | 5;
    type?: "secondary" | "success" | "warning" | "danger";
    strong?: boolean;
    italic?: boolean;
    code?: boolean;
    delete?: boolean;
    copyable?: boolean;
  };

  if (level) {
    return (
      <Title
        level={level}
        style={schema.style as React.CSSProperties}
      >
        {content}
      </Title>
    );
  }

  return (
    <Text
      type={type}
      strong={strong}
      italic={italic}
      code={code}
      delete={deleteStyle}
      copyable={copyable}
      style={schema.style as React.CSSProperties}
    >
      {content}
    </Text>
  );
};

/**
 * 分割线组件，基于 Ant Design Divider。
 */
export const DynamicDivider: React.FC<DynamicUIProps> = ({ schema }) => {
  const { text, orientation: titlePlacement = "center" as const, plain = false, dashed = false } = schema.props as {
    text?: string;
    orientation?: "left" | "right" | "center";
    plain?: boolean;
    dashed?: boolean;
  };

  return (
    <Divider
      titlePlacement={titlePlacement}
      plain={plain}
      dashed={dashed}
      style={schema.style as React.CSSProperties}
    >
      {text}
    </Divider>
  );
};

/**
 * 进度条组件，基于 Ant Design Progress。
 */
export const DynamicProgress: React.FC<DynamicUIProps> = ({ schema }) => {
  const {
    percent = 0,
    type = "line",
    status,
    showInfo = true,
    strokeColor,
  } = schema.props as {
    percent?: number;
    type?: "line" | "circle" | "dashboard";
    status?: "success" | "exception" | "normal" | "active";
    showInfo?: boolean;
    strokeColor?: string;
  };

  return (
    <Progress
      percent={percent}
      type={type}
      status={status}
      showInfo={showInfo}
      strokeColor={strokeColor}
      style={schema.style as React.CSSProperties}
    />
  );
};

/**
 * 标签组件，基于 Ant Design Tag。
 */
export const DynamicTag: React.FC<DynamicUIProps> = ({ schema }) => {
  const { text = "", color, closable = false } = schema.props as {
    text?: string;
    color?: string;
    closable?: boolean;
  };

  return (
    <Tag
      color={color}
      closable={closable}
      style={schema.style as React.CSSProperties}
    >
      {text}
    </Tag>
  );
};

/**
 * 图片组件，基于 Ant Design Image。
 */
export const DynamicImage: React.FC<DynamicUIProps> = ({ schema }) => {
  const {
    src = "",
    alt = "",
    width,
    height,
    preview = true,
    fallback,
  } = schema.props as {
    src?: string;
    alt?: string;
    width?: number;
    height?: number;
    preview?: boolean;
    fallback?: string;
  };

  return (
    <Image
      src={src}
      alt={alt}
      width={width}
      height={height}
      preview={preview}
      fallback={fallback}
      style={schema.style as React.CSSProperties}
    />
  );
};
