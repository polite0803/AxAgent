declare module "@xterm/xterm" {
  export class Terminal {
    constructor(options?: Record<string, unknown>);
    open(container: HTMLElement): void;
    onData(callback: (data: string) => void): void;
    onResize(callback: (size: { cols: number; rows: number }) => void): void;
    write(data: string): void;
    clear(): void;
    dispose(): void;
    loadAddon(addon: unknown): void;
  }
}

declare module "@xterm/addon-fit" {
  import type { Terminal } from "@xterm/xterm";
  export class FitAddon {
    constructor();
    fit(): void;
    activate(terminal: Terminal): void;
    dispose(): void;
  }
}

declare module "@xterm/addon-web-links" {
  export class WebLinksAddon {
    constructor();
  }
}

declare module "@xterm/xterm/css/xterm.css" {
  const content: string;
  export default content;
}
