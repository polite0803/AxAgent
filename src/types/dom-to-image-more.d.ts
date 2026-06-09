declare module "dom-to-image-more" {
  interface DomToImageOptions {
    bgColor?: string;
    width?: number;
    height?: number;
    scale?: number;
    style?: Record<string, string>;
    filter?: (node: HTMLElement) => boolean;
    quality?: number;
    imagePlaceholder?: string;
    cacheBust?: boolean;
  }

  export function toPng(node: HTMLElement, options?: DomToImageOptions): Promise<string>;
  export function toJpeg(node: HTMLElement, options?: DomToImageOptions): Promise<string>;
  export function toBlob(node: HTMLElement, options?: DomToImageOptions): Promise<Blob>;
  export function toPixelData(node: HTMLElement, options?: DomToImageOptions): Promise<Uint8ClampedArray>;
  export function toSvg(node: HTMLElement, options?: DomToImageOptions): Promise<string>;
}
