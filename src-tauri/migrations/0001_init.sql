-- CanvasClip — esquema inicial (user_version = 1)
--
-- Principios:
--  * Una captura = un archivo original + una miniatura, sin importar en
--    cuántos pizarrones se use. Los pizarrones referencian capturas por id.
--  * ON DELETE RESTRICT en board_objects.capture_id: la base de datos impide
--    borrar una captura en uso por accidente. El borrado permanente elimina
--    primero sus referencias, de forma explícita y dentro de una transacción.
--  * Las notas viven en board_objects (kind = 'note', texto y color en props).

CREATE TABLE captures (
    id            TEXT PRIMARY KEY,
    hash          TEXT NOT NULL UNIQUE,          -- blake3 de (ancho, alto, píxeles RGBA)
    file_name     TEXT NOT NULL,                 -- relativo a originals/
    thumb_name    TEXT NOT NULL,                 -- relativo a thumbs/
    mime          TEXT NOT NULL,
    width         INTEGER NOT NULL,
    height        INTEGER NOT NULL,
    thumb_width   INTEGER NOT NULL,
    thumb_height  INTEGER NOT NULL,
    byte_size     INTEGER NOT NULL,
    source        TEXT NOT NULL,                 -- 'clipboard' | 'paste'
    favorite      INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL               -- unix ms
);
CREATE INDEX idx_captures_created ON captures (created_at DESC);
CREATE INDEX idx_captures_favorite ON captures (favorite) WHERE favorite = 1;

CREATE TABLE boards (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    camera_x    REAL NOT NULL DEFAULT 0,
    camera_y    REAL NOT NULL DEFAULT 0,
    zoom        REAL NOT NULL DEFAULT 1
);

CREATE TABLE board_groups (
    id          TEXT PRIMARY KEY,
    board_id    TEXT NOT NULL REFERENCES boards (id) ON DELETE CASCADE,
    created_at  INTEGER NOT NULL
);
CREATE INDEX idx_groups_board ON board_groups (board_id);

CREATE TABLE board_objects (
    id          TEXT PRIMARY KEY,
    board_id    TEXT NOT NULL REFERENCES boards (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('image', 'note')),
    capture_id  TEXT REFERENCES captures (id) ON DELETE RESTRICT,
    x           REAL NOT NULL,
    y           REAL NOT NULL,
    width       REAL NOT NULL,
    height      REAL NOT NULL,
    z           INTEGER NOT NULL DEFAULT 0,
    group_id    TEXT REFERENCES board_groups (id) ON DELETE SET NULL,
    props       TEXT NOT NULL DEFAULT '{}',
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    CHECK ((kind = 'image' AND capture_id IS NOT NULL) OR kind = 'note')
);
CREATE INDEX idx_objects_board ON board_objects (board_id);
CREATE INDEX idx_objects_capture ON board_objects (capture_id) WHERE capture_id IS NOT NULL;

CREATE TABLE settings (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);
