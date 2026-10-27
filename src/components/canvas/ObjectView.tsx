import { ImageOff } from "lucide-react";
import { memo, useEffect, useState } from "react";

import { originalUrl, thumbUrl } from "@/services/images";
import type { BoardObject, Capture } from "@/types/models";

import { NoteView } from "../notes/NoteView";

interface Props {
  obj: BoardObject;
  capture: Capture | undefined;
  selected: boolean;
  editing: boolean;
  /** Mostrar el original además de la miniatura (zoom alto). */
  hiRes: boolean;
}

/** Un objeto del pizarrón en coordenadas del mundo. */
export const ObjectView = memo(function ObjectView({ obj, capture, selected, editing, hiRes }: Props) {
  return (
    <div
      className={`obj obj-${obj.kind} ${selected ? "is-selected" : ""} ${editing ? "is-editing" : ""}`}
      data-object-id={obj.id}
      style={{ left: obj.x, top: obj.y, width: obj.width, height: obj.height, zIndex: obj.z }}
    >
      {obj.kind === "image" ? <ImageContent capture={capture} hiRes={hiRes} /> : <NoteView obj={obj} editing={editing} />}
    </div>
  );
});

function ImageContent({ capture, hiRes }: { capture: Capture | undefined; hiRes: boolean }) {
  const [hiLoaded, setHiLoaded] = useState(false);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    if (!hiRes) setHiLoaded(false);
  }, [hiRes]);

  if (!capture || failed) {
    return (
      <div className="obj-missing">
        <ImageOff size={20} strokeWidth={1.5} />
        <span>Imagen no disponible</span>
      </div>
    );
  }
  return (
    <>
      {/* La miniatura queda debajo: el original aparece encima cuando termina de cargar. */}
      <img className="obj-img" src={thumbUrl(capture.id)} alt="" draggable={false} decoding="async" onError={() => setFailed(true)} />
      {hiRes && (
        <img
          className={`obj-img obj-img-hi ${hiLoaded ? "is-loaded" : ""}`}
          src={originalUrl(capture.id)}
          alt=""
          draggable={false}
          decoding="async"
          onLoad={() => setHiLoaded(true)}
        />
      )}
    </>
  );
}
