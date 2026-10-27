//! Pruebas de integración del núcleo: historial, retención, pizarrones y recuperación.

use std::time::Duration;

use crate::captures::Filter;
use crate::imaging::tests::sample_png;
use crate::imaging::{rgba_to_dib, ImagePayload};
use crate::ingest::{apply_retention, ingest};
use crate::maintenance::{copy_data_dir, reconcile};
use crate::models::{BoardChanges, BoardObject, Group};
use crate::{CoreError, Db, Storage};

struct Env {
    _dir: tempfile::TempDir,
    db: Db,
    st: Storage,
}

fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let st = Storage::new(dir.path().join("data")).unwrap();
    let db = Db::open(&st.db_path()).unwrap();
    Env { _dir: dir, db, st }
}

fn add(e: &Env, seed: u8, limit: u32) -> crate::ingest::IngestResult {
    let r = ingest(&e.db, &e.st, ImagePayload::Png(sample_png(20, 10, seed)), "clipboard", limit)
        .unwrap();
    // Garantiza created_at distintos y orden estable.
    std::thread::sleep(Duration::from_millis(2));
    r
}

fn image_obj(board: &str, capture: &str, id: &str) -> BoardObject {
    BoardObject {
        id: id.into(),
        board_id: board.into(),
        kind: "image".into(),
        capture_id: Some(capture.into()),
        x: 0.0,
        y: 0.0,
        width: 20.0,
        height: 10.0,
        z: 0,
        group_id: None,
        props: serde_json::json!({}),
        created_at: 0,
        updated_at: 0,
    }
}

#[test]
fn ingest_saves_files_and_dedupes() {
    let e = env();
    let r = add(&e, 1, 500);
    assert!(!r.duplicate);
    let files = e.db.capture_files(&r.capture.id).unwrap();
    assert!(e.st.original_path(&files.file_name).unwrap().exists());
    assert!(e.st.thumb_path(&files.thumb_name).unwrap().exists());
    assert_eq!((r.capture.width, r.capture.height), (20, 10));

    // Misma imagen otra vez (p. ej., CanvasClip la volvió a copiar): no se duplica.
    let again = add(&e, 1, 500);
    assert!(again.duplicate);
    assert_eq!(again.capture.id, r.capture.id);

    // Mismos píxeles llegando como DIB en vez de PNG: también es duplicado.
    let rgba = crate::imaging::load_rgba(&sample_png(20, 10, 1)).unwrap();
    let dib = ingest(&e.db, &e.st, ImagePayload::Dib(rgba_to_dib(&rgba)), "clipboard", 500).unwrap();
    assert!(dib.duplicate);
    assert_eq!(e.db.list_captures(Filter::All, 100, 0).unwrap().len(), 1);
}

#[test]
fn newest_first() {
    let e = env();
    let a = add(&e, 1, 500).capture.id;
    let b = add(&e, 2, 500).capture.id;
    let list = e.db.list_captures(Filter::All, 10, 0).unwrap();
    assert_eq!(list.iter().map(|c| c.id.clone()).collect::<Vec<_>>(), vec![b, a]);
}

#[test]
fn retention_keeps_favorites_and_board_items() {
    let e = env();
    let board = e.db.ensure_default_board().unwrap();
    let fav = add(&e, 1, 1000).capture.id;
    let used = add(&e, 2, 1000).capture.id;
    let mut plain = vec![];
    for s in 3..10 {
        plain.push(add(&e, s, 1000).capture.id);
    }
    e.db.set_favorite(&fav, true).unwrap();
    e.db.save_board_changes(&BoardChanges {
        board_id: board.clone(),
        upsert_objects: vec![image_obj(&board, &used, "o1")],
        ..Default::default()
    })
    .unwrap();

    // Límite 3: quedan las 3 no protegidas más recientes + las 2 protegidas.
    let pruned = apply_retention(&e.db, &e.st, 3).unwrap();
    assert_eq!(pruned.len(), 4);
    let ids: Vec<String> =
        e.db.list_captures(Filter::All, 100, 0).unwrap().into_iter().map(|c| c.id).collect();
    assert!(ids.contains(&fav));
    assert!(ids.contains(&used));
    assert_eq!(ids.len(), 5);
    for old in &plain[..4] {
        assert!(!ids.contains(old));
    }
    // Los archivos de las podadas se borraron.
    let files = std::fs::read_dir(e.st.originals_dir()).unwrap().count();
    assert_eq!(files, 5);
}

#[test]
fn protected_above_limit_are_all_kept() {
    let e = env();
    let mut ids = vec![];
    for s in 0..5 {
        let id = add(&e, s, 1000).capture.id;
        e.db.set_favorite(&id, true).unwrap();
        ids.push(id);
    }
    apply_retention(&e.db, &e.st, 2).unwrap();
    assert_eq!(e.db.list_captures(Filter::All, 100, 0).unwrap().len(), 5);
    // Una nueva no protegida con límite 2 también se conserva (cuenta 1 de 2).
    add(&e, 99, 2);
    assert_eq!(e.db.list_captures(Filter::All, 100, 0).unwrap().len(), 6);
}

#[test]
fn delete_in_use_requires_force() {
    let e = env();
    let board = e.db.ensure_default_board().unwrap();
    let id = add(&e, 1, 500).capture.id;
    e.db.save_board_changes(&BoardChanges {
        board_id: board.clone(),
        upsert_objects: vec![image_obj(&board, &id, "o1")],
        ..Default::default()
    })
    .unwrap();
    assert_eq!(e.db.get_capture(&id).unwrap().usage_count, 1);
    assert!(matches!(e.db.delete_capture(&id, false), Err(CoreError::InUse(1))));
    // El RESTRICT de la base también protege ante un borrado directo.
    assert!(e.db.conn().execute("DELETE FROM captures WHERE id = ?1", [&id]).is_err());

    let (removed, boards) = e.db.delete_capture(&id, true).unwrap();
    assert_eq!(boards, vec![board.clone()]);
    crate::ingest::remove_files(&e.st, removed);
    assert!(e.db.load_board(&board).unwrap().objects.is_empty());
    assert_eq!(std::fs::read_dir(e.st.originals_dir()).unwrap().count(), 0);
}

#[test]
fn removing_from_board_keeps_history() {
    let e = env();
    let board = e.db.ensure_default_board().unwrap();
    let id = add(&e, 1, 500).capture.id;
    let ch = |up: Vec<BoardObject>, del: Vec<String>| BoardChanges {
        board_id: board.clone(),
        upsert_objects: up,
        delete_objects: del,
        ..Default::default()
    };
    e.db.save_board_changes(&ch(vec![image_obj(&board, &id, "o1")], vec![])).unwrap();
    e.db.save_board_changes(&ch(vec![], vec!["o1".into()])).unwrap();
    assert!(e.db.get_capture(&id).is_ok());
    assert_eq!(e.db.get_capture(&id).unwrap().usage_count, 0);
}

#[test]
fn same_capture_in_many_boards_single_file() {
    let e = env();
    let a = e.db.ensure_default_board().unwrap();
    let b = e.db.create_board("Trabajo").unwrap().id;
    let id = add(&e, 1, 500).capture.id;
    for (board, oid) in [(&a, "x1"), (&b, "x2")] {
        e.db.save_board_changes(&BoardChanges {
            board_id: board.clone(),
            upsert_objects: vec![image_obj(board, &id, oid)],
            ..Default::default()
        })
        .unwrap();
    }
    assert_eq!(e.db.get_capture(&id).unwrap().usage_count, 2);
    assert_eq!(std::fs::read_dir(e.st.originals_dir()).unwrap().count(), 1);
}

#[test]
fn boards_lifecycle_groups_and_duplicate() {
    let e = env();
    let first = e.db.ensure_default_board().unwrap();
    assert_eq!(e.db.ensure_default_board().unwrap(), first, "no crea un segundo predeterminado");
    let id = add(&e, 1, 500).capture.id;
    let g = Group { id: "g1".into(), board_id: first.clone(), created_at: 0 };
    let mut o1 = image_obj(&first, &id, "o1");
    o1.group_id = Some("g1".into());
    let mut note = image_obj(&first, &id, "n1");
    note.kind = "note".into();
    note.capture_id = None;
    note.group_id = Some("g1".into());
    note.props = serde_json::json!({ "text": "hola", "color": "sand" });
    e.db.save_board_changes(&BoardChanges {
        board_id: first.clone(),
        upsert_objects: vec![o1, note],
        upsert_groups: vec![g],
        ..Default::default()
    })
    .unwrap();
    e.db.save_camera(&first, 10.0, -5.0, 1.5).unwrap();

    let doc = e.db.load_board(&first).unwrap();
    assert_eq!(doc.objects.len(), 2);
    assert_eq!(doc.groups.len(), 1);
    assert_eq!(doc.board.zoom, 1.5);
    assert_eq!(doc.objects.iter().find(|o| o.id == "n1").unwrap().props["text"], "hola");

    let dup = e.db.duplicate_board(&first).unwrap();
    let dup_doc = e.db.load_board(&dup.id).unwrap();
    assert_eq!(dup_doc.objects.len(), 2);
    assert_eq!(dup_doc.groups.len(), 1);
    let new_gid = &dup_doc.groups[0].id;
    assert!(dup_doc.objects.iter().all(|o| o.group_id.as_ref() == Some(new_gid)));
    assert_eq!(dup_doc.board.camera_x, 10.0);

    let renamed = e.db.rename_board(&dup.id, "  Referencias  ").unwrap();
    assert_eq!(renamed.name, "Referencias");
    assert!(e.db.rename_board(&dup.id, "   ").is_err());

    let summaries = e.db.list_boards().unwrap();
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].object_count, 2);
    assert_eq!(summaries[0].preview.iter().filter(|p| p.color.as_deref() == Some("sand")).count(), 1);

    // Borrar ambos deja siempre un pizarrón disponible.
    e.db.delete_board(&first).unwrap();
    let fallback = e.db.delete_board(&dup.id).unwrap();
    assert!(e.db.board_exists(&fallback).unwrap());
    // La captura sigue en el historial.
    assert!(e.db.get_capture(&id).is_ok());
}

#[test]
fn save_skips_objects_with_deleted_capture() {
    let e = env();
    let board = e.db.ensure_default_board().unwrap();
    let ok = add(&e, 1, 500).capture.id;
    let report = e
        .db
        .save_board_changes(&BoardChanges {
            board_id: board.clone(),
            upsert_objects: vec![image_obj(&board, &ok, "a"), image_obj(&board, "no-existe", "b")],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(report.skipped_objects, vec!["b".to_string()]);
    assert_eq!(e.db.load_board(&board).unwrap().objects.len(), 1);
}

#[test]
fn insert_capture_object_places_right_of_content() {
    let e = env();
    let board = e.db.ensure_default_board().unwrap();
    let id = add(&e, 1, 500).capture.id;
    let first = e.db.insert_capture_object(&board, &id, 200.0, 100.0).unwrap();
    assert_eq!((first.x, first.y), (-100.0, -50.0));
    let second = e.db.insert_capture_object(&board, &id, 200.0, 100.0).unwrap();
    assert_eq!(second.x, 100.0 + 40.0);
    assert!(second.z > first.z);
}

#[test]
fn reconcile_after_crash() {
    let e = env();
    let board = e.db.ensure_default_board().unwrap();
    let keep = add(&e, 1, 500).capture.id;
    let used = add(&e, 2, 500).capture.id;
    let lost = add(&e, 3, 500).capture.id;
    e.db.save_board_changes(&BoardChanges {
        board_id: board.clone(),
        upsert_objects: vec![image_obj(&board, &used, "o")],
        ..Default::default()
    })
    .unwrap();

    // Simula: .tmp a medio escribir, archivo huérfano, miniatura borrada,
    // originales perdidos (uno en uso y otro no).
    std::fs::write(e.st.originals_dir().join("x.png.tmp"), b"partial").unwrap();
    std::fs::write(e.st.originals_dir().join("orphan.png"), b"zzz").unwrap();
    let kf = e.db.capture_files(&keep).unwrap();
    std::fs::remove_file(e.st.thumb_path(&kf.thumb_name).unwrap()).unwrap();
    for id in [&used, &lost] {
        let f = e.db.capture_files(id).unwrap();
        std::fs::remove_file(e.st.original_path(&f.file_name).unwrap()).unwrap();
    }

    let r = reconcile(&e.db, &e.st, Duration::ZERO).unwrap();
    assert_eq!(r.tmp_removed, 1);
    assert_eq!(r.orphans_removed, 1);
    assert_eq!(r.thumbs_rebuilt, 1);
    assert_eq!(r.missing_rows_removed, 1);
    assert_eq!(r.missing_referenced, 1);
    assert!(e.db.get_capture(&lost).is_err());
    assert!(e.db.get_capture(&used).is_ok(), "la usada en un pizarrón no se pierde");
    let kf = e.db.capture_files(&keep).unwrap();
    assert!(e.st.thumb_path(&kf.thumb_name).unwrap().exists());

    // Una segunda pasada no encuentra nada más que hacer (salvo el faltante en uso).
    let again = reconcile(&e.db, &e.st, Duration::ZERO).unwrap();
    assert_eq!(again.orphans_removed + again.tmp_removed + again.thumbs_rebuilt, 0);
}

#[test]
fn data_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let id;
    {
        let st = Storage::new(dir.path()).unwrap();
        let db = Db::open(&st.db_path()).unwrap();
        id = ingest(&db, &st, ImagePayload::Png(sample_png(8, 8, 4)), "clipboard", 500)
            .unwrap()
            .capture
            .id;
        db.set_favorite(&id, true).unwrap();
        // Sin cierre ordenado: el Db se descarta sin checkpoint explícito.
    }
    let st = Storage::new(dir.path()).unwrap();
    let db = Db::open(&st.db_path()).unwrap();
    assert!(db.get_capture(&id).unwrap().favorite);
}

#[test]
fn move_data_dir() {
    let e = env();
    let board = e.db.ensure_default_board().unwrap();
    let id = add(&e, 1, 500).capture.id;
    e.db.insert_capture_object(&board, &id, 20.0, 10.0).unwrap();
    let dest = e._dir.path().join("nuevo");
    let to = copy_data_dir(&e.db, &e.st, &dest).unwrap();
    let copy = Db::open(&to.db_path()).unwrap();
    assert_eq!(copy.load_board(&board).unwrap().objects.len(), 1);
    let f = copy.capture_files(&id).unwrap();
    assert!(to.original_path(&f.file_name).unwrap().exists());
    // No se puede copiar dos veces sobre el mismo destino.
    assert!(copy_data_dir(&e.db, &e.st, &dest).is_err());
}

#[test]
fn settings_roundtrip_and_normalize() {
    let e = env();
    let mut s = e.db.load_settings().unwrap();
    assert_eq!(s.history_limit, 500);
    s.history_limit = 3;
    s.effects = "neon".into();
    let s = s.normalized();
    assert_eq!(s.history_limit, crate::settings::MIN_HISTORY);
    assert_eq!(s.effects, "subtle");
    e.db.save_settings(&s).unwrap();
    assert_eq!(e.db.load_settings().unwrap(), s);
    // JSON viejo sin campos nuevos: se completa con valores por defecto.
    e.db.conn()
        .execute("UPDATE settings SET value = '{\"historyLimit\": 900}' WHERE key = 'app'", [])
        .unwrap();
    let s = e.db.load_settings().unwrap();
    assert_eq!(s.history_limit, 900);
    assert_eq!(s.shortcut, "Alt+Shift+V");
    assert!(e.db.take_first_run().unwrap());
    assert!(!e.db.take_first_run().unwrap());
}
