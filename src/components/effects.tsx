import { useEffect, useRef } from "react";
import type { Annotation, EffectKind } from "../types";

type EffectAnnotation = Extract<Annotation, { kind: EffectKind }>;

type CommonProps = {
  ann: EffectAnnotation;
  imageUrl: string;
  imgWidth: number;
  imgHeight: number;
  scale: number;
};

function PixelateCanvas({ ann, imageUrl }: { ann: { x: number; y: number; width: number; height: number; strength: number }; imageUrl: string }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const img = new Image();
    img.onload = () => {
      c.width = Math.max(1, Math.round(ann.width));
      c.height = Math.max(1, Math.round(ann.height));
      const ctx = c.getContext("2d")!;
      const block = Math.max(2, ann.strength);
      const sw = Math.max(1, Math.round(ann.width / block));
      const sh = Math.max(1, Math.round(ann.height / block));
      const small = document.createElement("canvas");
      small.width = sw;
      small.height = sh;
      const sctx = small.getContext("2d")!;
      sctx.imageSmoothingEnabled = false;
      sctx.drawImage(img, ann.x, ann.y, ann.width, ann.height, 0, 0, sw, sh);
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(small, 0, 0, sw, sh, 0, 0, c.width, c.height);
    };
    img.src = imageUrl;
  }, [ann.x, ann.y, ann.width, ann.height, ann.strength, imageUrl]);
  return (
    <canvas
      ref={ref}
      style={{ position: "absolute", left: ann.x, top: ann.y, width: ann.width, height: ann.height }}
    />
  );
}

export function EffectRegion({ ann, imageUrl, imgWidth, imgHeight, scale }: CommonProps) {
  const base: React.CSSProperties = {
    position: "absolute",
    left: ann.x,
    top: ann.y,
    width: ann.width,
    height: ann.height,
  };
  if (ann.kind === "blur") {
    return (
      <div
        className="effect blur"
        style={{
          ...base,
          backgroundImage: `url(${imageUrl})`,
          backgroundSize: `${imgWidth}px ${imgHeight}px`,
          backgroundPosition: `-${ann.x}px -${ann.y}px`,
          filter: `blur(${ann.strength * scale}px)`,
        }}
      />
    );
  }
  if (ann.kind === "pixelate") return <PixelateCanvas ann={ann} imageUrl={imageUrl} />;
  if (ann.kind === "highlight") {
    return <div className="effect highlight" style={{ ...base, background: ann.color }} />;
  }
  return <div className="effect redact" style={base} />;
}
