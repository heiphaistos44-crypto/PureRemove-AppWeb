import { useCallback, useRef, useState } from "react";
import { cn } from "@/lib/utils";
import { ACCEPTED_EXTENSIONS } from "@/types";

interface DropZoneProps {
  onFiles: (files: File[]) => void;
  disabled?: boolean;
}

const ACCEPT_ATTR = ACCEPTED_EXTENSIONS.map((e) => `.${e}`).join(",");

function isImageFile(f: File): boolean {
  const ext = f.name.split(".").pop()?.toLowerCase() ?? "";
  return (
    (ACCEPTED_EXTENSIONS as readonly string[]).includes(ext) ||
    f.type.startsWith("image/")
  );
}

export function DropZone({ onFiles, disabled }: DropZoneProps) {
  const [isDragging, setIsDragging] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleFiles = useCallback(
    (list: FileList | null) => {
      if (disabled || !list) return;
      const images = Array.from(list).filter(isImageFile);
      if (images.length > 0) onFiles(images);
    },
    [disabled, onFiles]
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setIsDragging(false);
      handleFiles(e.dataTransfer.files);
    },
    [handleFiles]
  );

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  }, []);

  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center gap-6 w-full h-full rounded-2xl border-2 border-dashed transition-all duration-200 cursor-pointer select-none",
        isDragging
          ? "border-primary bg-primary/10 scale-[1.01]"
          : "border-border hover:border-primary/50 hover:bg-secondary/30",
        disabled && "opacity-50 pointer-events-none"
      )}
      onDragOver={handleDragOver}
      onDragEnter={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      onClick={() => inputRef.current?.click()}
    >
      <input
        ref={inputRef}
        type="file"
        multiple
        accept={ACCEPT_ATTR}
        className="hidden"
        onChange={(e) => {
          handleFiles(e.target.files);
          e.target.value = ""; // permet de re-sélectionner le même fichier
        }}
      />

      {/* Icône */}
      <div className={cn(
        "w-20 h-20 rounded-full flex items-center justify-center transition-all duration-200",
        isDragging ? "bg-primary/20" : "bg-secondary"
      )}>
        <svg className="w-10 h-10 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
          <path strokeLinecap="round" strokeLinejoin="round"
            d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5" />
        </svg>
      </div>

      {/* Texte */}
      <div className="text-center space-y-2 px-8">
        <p className="text-foreground font-semibold text-lg">
          {isDragging ? "Déposez vos images ici" : "Glissez vos images ici"}
        </p>
        <p className="text-muted-foreground text-sm">
          ou{" "}
          <span className="text-primary font-medium">parcourez vos fichiers</span>
          {" "}— ou collez avec{" "}
          <kbd className="px-2 py-0.5 rounded bg-secondary border border-border text-xs font-mono">Ctrl+V</kbd>
        </p>
        <p className="text-muted-foreground text-xs mt-1">
          PNG · JPG · WEBP · SVG · BMP · GIF · TIFF · ICO · TGA · HDR · QOI — fichier unique ou multiple
        </p>
      </div>
    </div>
  );
}
