export type BackgroundColor =
  | { type: "Transparent" }
  | { type: "White" }
  | { type: "Black" }
  | { type: "Color"; r: number; g: number; b: number };

export const ACCEPTED_EXTENSIONS = [
  "png", "jpg", "jpeg", "webp", "svg",
  "bmp", "gif", "tif", "tiff", "ico",
  "tga", "pnm", "pbm", "pgm", "ppm",
  "hdr", "ff", "qoi",
] as const;

export interface ProcessOptions {
  background: BackgroundColor;
}

export function isValidBackground(v: unknown): v is BackgroundColor {
  if (typeof v !== "object" || v === null) return false;
  const o = v as Record<string, unknown>;
  switch (o["type"]) {
    case "Transparent":
    case "White":
    case "Black":
      return true;
    case "Color":
      return typeof o["r"] === "number" && typeof o["g"] === "number" && typeof o["b"] === "number";
    default:
      return false;
  }
}

export type ItemStatus = "pending" | "processing" | "done" | "error";

export interface ImageItem {
  id: string;
  name: string;
  file?: File;
  status: ItemStatus;
  /** Object URL du PNG résultat (compatible <img src>) */
  resultDataUrl?: string;
  /** Blob résultat — pour le ZIP et la copie presse-papier */
  resultBlob?: Blob;
  error?: string;
}

export type AppMode = "idle" | "single" | "batch";
