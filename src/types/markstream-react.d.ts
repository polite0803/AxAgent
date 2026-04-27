import "markstream-react";

declare module "markstream-react" {
  // Extend MermaidBlockNodeProps to include custom render props
  interface MermaidBlockNodeProps {
    renderHeaderActions?: (ctx: MermaidBlockActionContext) => React.ReactNode;
    renderModeToggle?: (ctx: MermaidBlockActionContext) => React.ReactNode;
    renderZoomControls?: (ctx: MermaidBlockActionContext) => React.ReactNode;
  }

  // Extend InfographicBlockNodeProps to include custom render props
  interface InfographicBlockNodeProps {
    renderHeaderActions?: (ctx: InfographicBlockActionContext) => React.ReactNode;
    renderModeToggle?: (ctx: InfographicBlockActionContext) => React.ReactNode;
    renderZoomControls?: (ctx: InfographicBlockActionContext) => React.ReactNode;
  }

  // Custom action context types (not exported by markstream-react)
  interface MermaidBlockActionContext {
    collapsed: boolean;
    toggleCollapse: () => void;
    copied: boolean;
    copy: () => void;
    mermaidAvailable: boolean;
    isExportDisabled: boolean;
    exportSvg: () => void;
    modalOpen: boolean;
    toggleFullscreen: () => void;
    showSource: boolean;
    switchMode: () => void;
    zoomIn: () => void;
    zoomOut: () => void;
    resetZoom: () => void;
    zoom: number;
  }

  interface InfographicBlockActionContext {
    collapsed: boolean;
    toggleCollapse: () => void;
    copied: boolean;
    copy: () => void;
    isExportDisabled: boolean;
    exportSvg: () => void;
    modalOpen: boolean;
    toggleFullscreen: () => void;
    showSource: boolean;
    switchMode: () => void;
  }
}
