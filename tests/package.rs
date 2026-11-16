//! Publication integrity, sharding and offline package contracts.
use ghostline_items::{model::Catalogue, publish};
use serde_json::json;
use std::collections::BTreeMap;

fn catalogue() -> Catalogue {
    serde_json::from_value(json!({"format":"ghostline-items","version":1,"sources":{},"items":[],"assets":{},"controls":[],"choices":[],"selections":{},"customization_metadata":{},"coverage":{},"diagnostics":[]})).unwrap()
}

#[test]
fn publication_prunes_old_renders_and_keeps_referenced_media() {
    let temp = tempfile::tempdir().unwrap();
    publish::initialize(temp.path()).unwrap();
    let mut c = catalogue();
    let id = ghostline_items::build::add_selection(
        &mut c,
        ghostline_red::assets::ResourceKey::parse("base/test.app"),
        Some("default".into()),
        json!({}),
    );
    let s = c.selections.get_mut(&id).unwrap();
    s.resolution = "resolved".into();
    s.preview_state = "ready".into();
    s.media.push(ghostline_items::model::Media {
        path: "media/current.png".into(),
        view: "hero".into(),
        bytes: 3,
        size: 128,
    });
    std::fs::create_dir_all(temp.path().join("media")).unwrap();
    std::fs::write(temp.path().join("media/current.png"), b"png").unwrap();
    std::fs::write(temp.path().join("media/old-broken.png"), b"broken").unwrap();
    publish::write(&mut c, temp.path(), "").unwrap();
    assert_eq!(publish::prune_media(&c, temp.path()).unwrap(), 1);
    assert!(!temp.path().join("media/old-broken.png").exists());
    assert_eq!(
        std::fs::read(temp.path().join("media/current.png")).unwrap(),
        b"png"
    );
    publish::verify(temp.path()).unwrap();
}

#[test]
fn shards_round_trip_unicode_and_detect_browser_corruption() {
    let temp = tempfile::tempdir().unwrap();
    publish::initialize(temp.path()).unwrap();
    let mut c = catalogue();
    for n in 0..129_u64 {
        c.items.push(serde_json::from_value(json!({"id":format!("item:{n}"),"native_id":n.to_string(),"native_name":format!("Items.Example{n}"),"label":"Étoile / 星","native_class":"gamedataItem_Record","category":null,"item_type":null,"properties":{},"localization":{},"selections":[],"diagnostics":[]})).unwrap());
    }
    publish::write(&mut c, temp.path(), "").unwrap();
    let descriptor_bytes = std::fs::read(temp.path().join("data/catalogue.json")).unwrap();
    publish::verify(temp.path()).unwrap();
    let loaded = publish::load(temp.path()).unwrap();
    assert_eq!(loaded.items.len(), 129);
    assert_eq!(loaded.items[0].label, "Étoile / 星");
    let descriptor: serde_json::Value =
        serde_json::from_slice(&std::fs::read(temp.path().join("data/catalogue.json")).unwrap())
            .unwrap();
    assert_eq!(
        descriptor["collections"]["items"].as_array().unwrap().len(),
        2
    );
    publish::write(&mut c, temp.path(), "").unwrap();
    assert_eq!(
        std::fs::read(temp.path().join("data/catalogue.json")).unwrap(),
        descriptor_bytes
    );
    std::fs::write(temp.path().join("assets/app.js"), "corrupt").unwrap();
    assert!(
        publish::verify(temp.path())
            .unwrap_err()
            .to_string()
            .contains("length mismatch")
    );
}

#[test]
fn refuses_unowned_output_and_orphaned_selections() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("keep.txt"), "owned by someone else").unwrap();
    assert!(publish::initialize(temp.path()).is_err());
    let mut c = catalogue();
    c.items.push(serde_json::from_value(json!({"id":"item:1","native_id":"1","native_name":null,"label":"Missing","native_class":"gamedataItem_Record","category":null,"item_type":null,"properties":{},"localization":{},"selections":["base/missing.app#default"],"diagnostics":[]})).unwrap());
    assert!(
        publish::validate(&c, temp.path())
            .unwrap_err()
            .to_string()
            .contains("missing item selection")
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("keep.txt")).unwrap(),
        "owned by someone else"
    );
}

#[test]
fn ready_media_requires_matching_content_and_safe_path() {
    let temp = tempfile::tempdir().unwrap();
    let mut c = catalogue();
    let selection=serde_json::from_value(json!({"id":"base/test.app#default","resource":{"Hash":"18446744073709551615"},"appearance":"default","context":{},"components":[],"resolution":"resolved","preview_state":"ready","diagnostics":[],"media":[]})).unwrap();
    c.selections = BTreeMap::from([("base/test.app#default".into(), selection)]);
    assert!(
        publish::validate(&c, temp.path())
            .unwrap_err()
            .to_string()
            .contains("no media")
    );
    c.selections
        .get_mut("base/test.app#default")
        .unwrap()
        .media
        .push(
            serde_json::from_value(
                json!({"path":"../outside.png","view":"hero","bytes":3,"size":128}),
            )
            .unwrap(),
        );
    assert!(
        publish::validate(&c, temp.path())
            .unwrap_err()
            .to_string()
            .contains("unsafe media path")
    );
}
