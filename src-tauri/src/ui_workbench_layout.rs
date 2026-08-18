//! Validated layout-review snapshots for registered UI Workbench pages.

use crate::ui_workbench_catalog::WorkbenchCatalog;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const LAYOUT_REVIEW_ARTIFACT_TYPE: &str = "ui_layout_review";
const LAYOUT_REVIEW_SCHEMA_VERSION: u32 = 1;
const LAYOUT_REVIEW_STATUS: &str = "pending_chat_confirmation";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutPageSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutReviewRequest {
    #[serde(default)]
    pub workflow_task_id: Option<String>,
    pub page_size: LayoutPageSize,
    pub nodes: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayoutReviewSource {
    pub session_file: String,
    pub session_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_task_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayoutChangeSummary {
    pub changed_node_count: usize,
    pub added: Vec<String>,
    pub deleted: Vec<String>,
    pub moved: Vec<String>,
    pub resized: Vec<String>,
    pub reparented: Vec<String>,
    pub z_order_changed: Vec<String>,
    pub classification_changed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutReview {
    pub artifact_type: String,
    pub schema_version: u32,
    pub status: String,
    pub page_id: String,
    pub revision: u64,
    pub saved_at: String,
    pub source: LayoutReviewSource,
    pub page_size: LayoutPageSize,
    pub nodes: Vec<Value>,
    pub change_summary: LayoutChangeSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayoutReviewSaveResult {
    pub page_id: String,
    pub revision: u64,
    pub saved_at: String,
    pub status: String,
    pub content_sha256: String,
}

pub fn save_layout_review(
    catalog: &WorkbenchCatalog,
    page_id: &str,
    request: LayoutReviewRequest,
    now_unix_ms: u64,
) -> Result<LayoutReviewSaveResult, String> {
    let page = catalog
        .pages
        .iter()
        .find(|page| page.page_id == page_id)
        .ok_or_else(|| format!("unknown UI Workbench page: {page_id}"))?;
    let session_path = page.session_dir.join("session.json");
    let session_bytes = fs::read(&session_path)
        .map_err(|error| format!("could not read {}: {error}", session_path.display()))?;
    let session: Value = serde_json::from_slice(&session_bytes)
        .map_err(|error| format!("invalid {}: {error}", session_path.display()))?;
    if session.get("page_id").and_then(Value::as_str) != Some(page_id) {
        return Err(format!(
            "session page_id does not match registered page: {page_id}"
        ));
    }
    validate_request(&request)?;

    let review_path = page.session_dir.join("layout-review.json");
    let revision = next_revision_or_recover(&review_path, page_id, now_unix_ms)?;
    let saved_at = format_saved_at(now_unix_ms)?;
    let review = LayoutReview {
        artifact_type: LAYOUT_REVIEW_ARTIFACT_TYPE.into(),
        schema_version: LAYOUT_REVIEW_SCHEMA_VERSION,
        status: LAYOUT_REVIEW_STATUS.into(),
        page_id: page_id.into(),
        revision,
        saved_at: saved_at.clone(),
        source: LayoutReviewSource {
            session_file: "session.json".into(),
            session_sha256: hex_sha256(&session_bytes),
            workflow_task_id: request.workflow_task_id,
        },
        page_size: request.page_size,
        change_summary: build_change_summary(&session, &request.nodes),
        nodes: request.nodes,
    };
    let json = serde_json::to_vec_pretty(&review)
        .map_err(|error| format!("could not serialize layout review: {error}"))?;
    write_snapshot(&review_path, &json)?;

    Ok(LayoutReviewSaveResult {
        page_id: page_id.into(),
        revision,
        saved_at,
        status: LAYOUT_REVIEW_STATUS.into(),
        content_sha256: hex_sha256(&json),
    })
}

pub fn load_layout_review(
    catalog: &WorkbenchCatalog,
    page_id: &str,
) -> Result<Option<LayoutReview>, String> {
    let page = catalog
        .pages
        .iter()
        .find(|page| page.page_id == page_id)
        .ok_or_else(|| format!("unknown UI Workbench page: {page_id}"))?;
    let path = page.session_dir.join("layout-review.json");
    if !path.is_file() {
        return Ok(None);
    }
    let raw =
        fs::read(&path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let review: LayoutReview = serde_json::from_slice(&raw)
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    if review.artifact_type != LAYOUT_REVIEW_ARTIFACT_TYPE
        || review.schema_version != LAYOUT_REVIEW_SCHEMA_VERSION
    {
        return Err(format!(
            "unsupported layout review schema: {} {}",
            review.artifact_type, review.schema_version
        ));
    }
    if review.page_id != page_id {
        return Err(format!(
            "layout review page_id does not match registered page: {page_id}"
        ));
    }
    Ok(Some(review))
}

fn validate_request(request: &LayoutReviewRequest) -> Result<(), String> {
    if !request.page_size.width.is_finite()
        || !request.page_size.height.is_finite()
        || request.page_size.width <= 0.0
        || request.page_size.height <= 0.0
    {
        return Err("layout page size must be finite and greater than zero".into());
    }
    let mut ids = HashSet::new();
    let mut parents = HashMap::new();
    for node in &request.nodes {
        let object = node
            .as_object()
            .ok_or_else(|| "layout nodes must contain objects".to_string())?;
        let id = required_node_string(object.get("id"), "id", "layout node id must be non-empty")?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("duplicate node id: {id}"));
        }
        required_node_string(
            object.get("category"),
            "category",
            &format!("node {id} must contain a non-empty category"),
        )?;
        validate_bounds(id, object.get("bounds"))?;
        validate_extraction(id, object.get("extraction"))?;
        validate_z_index(id, object.get("z_index"))?;
        validate_enum(
            id,
            "node_kind",
            object.get("node_kind"),
            &["composite", "skin", "artwork", "native", "interaction"],
        )?;
        validate_enum(
            id,
            "render_mode",
            object.get("render_mode"),
            &["bitmap", "outline", "ghost", "assembly", "hidden"],
        )?;
        validate_visual_assets(id, object.get("visual_assets"))?;
        let parent = object
            .get("parent_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        if parent.as_deref() == Some(id) {
            return Err(format!("node {id} cannot parent itself"));
        }
        parents.insert(id.to_owned(), parent);
    }
    for (id, parent) in &parents {
        if let Some(parent) = parent {
            if !ids.contains(parent) {
                return Err(format!("node {id} references missing parent {parent}"));
            }
        }
    }
    for id in &ids {
        let mut seen = HashSet::new();
        let mut current = Some(id.as_str());
        while let Some(node_id) = current {
            if !seen.insert(node_id.to_owned()) {
                return Err(format!("parent cycle detected at node {node_id}"));
            }
            current = parents.get(node_id).and_then(|parent| parent.as_deref());
        }
    }
    Ok(())
}

fn required_node_string<'a>(
    value: Option<&'a Value>,
    _field: &str,
    message: &str,
) -> Result<&'a str, String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| message.to_owned())
}

fn validate_bounds(id: &str, value: Option<&Value>) -> Result<(), String> {
    let bounds = value
        .and_then(Value::as_object)
        .ok_or_else(|| format!("node {id} must contain bounds"))?;
    for field in ["x", "y", "width", "height"] {
        let number = bounds
            .get(field)
            .and_then(Value::as_f64)
            .filter(|number| number.is_finite())
            .ok_or_else(|| format!("node {id} bounds {field} must be finite"))?;
        if matches!(field, "width" | "height") && number <= 0.0 {
            return Err(format!(
                "node {id} bounds {field} must be finite and greater than zero"
            ));
        }
    }
    Ok(())
}

fn validate_extraction(id: &str, value: Option<&Value>) -> Result<(), String> {
    let extraction = value
        .and_then(Value::as_object)
        .ok_or_else(|| format!("node {id} must contain a valid extraction object"))?;
    let mode = required_node_string(
        extraction.get("mode"),
        "mode",
        &format!("node {id} extraction mode must be non-empty"),
    )?;
    if !["native", "extract_artwork", "reconstruct_skin", "composite"].contains(&mode) {
        return Err(format!("node {id} has unsupported extraction mode: {mode}"));
    }
    required_node_string(
        extraction.get("target_component_id"),
        "target_component_id",
        &format!("node {id} extraction target_component_id must be non-empty"),
    )?;
    Ok(())
}

fn validate_z_index(id: &str, value: Option<&Value>) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    let valid = value
        .as_i64()
        .is_some_and(|number| i32::try_from(number).is_ok());
    if valid {
        Ok(())
    } else {
        Err(format!("node {id} z_index must be a signed 32-bit integer"))
    }
}

fn validate_enum(
    id: &str,
    field: &str,
    value: Option<&Value>,
    allowed: &[&str],
) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    let candidate = value
        .as_str()
        .ok_or_else(|| format!("node {id} has unsupported {field}"))?;
    if allowed.contains(&candidate) {
        Ok(())
    } else {
        Err(format!("node {id} has unsupported {field}: {candidate}"))
    }
}

fn validate_visual_assets(id: &str, value: Option<&Value>) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    let assets = value
        .as_object()
        .ok_or_else(|| format!("node {id} visual_assets must be an object"))?;
    for path in assets.values().filter_map(Value::as_str) {
        if path.is_empty() || path == "__source__" || is_native_asset_reference(path) {
            continue;
        }
        let relative = Path::new(path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!("node {id} has invalid visual asset path: {path}"));
        }
    }
    Ok(())
}

fn is_native_asset_reference(path: &str) -> bool {
    ["/Game/", "/RedCliff/", "/Script/", "/Engine/"]
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

fn next_revision_or_recover(path: &Path, page_id: &str, now_unix_ms: u64) -> Result<u64, String> {
    if !path.is_file() {
        return Ok(1);
    }
    let raw =
        fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let review = match serde_json::from_slice::<LayoutReview>(&raw) {
        Ok(review) => review,
        Err(_) => {
            let invalid = path.with_file_name(format!("layout-review.invalid-{now_unix_ms}.json"));
            fs::rename(path, &invalid).map_err(|error| {
                format!(
                    "could not retain invalid layout review as {}: {error}",
                    invalid.display()
                )
            })?;
            return Ok(1);
        }
    };
    if review.artifact_type != LAYOUT_REVIEW_ARTIFACT_TYPE
        || review.schema_version != LAYOUT_REVIEW_SCHEMA_VERSION
    {
        return Err(format!(
            "unsupported layout review schema: {} {}",
            review.artifact_type, review.schema_version
        ));
    }
    if review.page_id != page_id {
        return Err(format!(
            "layout review page_id does not match registered page: {page_id}"
        ));
    }
    Ok(review.revision + 1)
}

fn build_change_summary(session: &Value, nodes: &[Value]) -> LayoutChangeSummary {
    let source_nodes = session
        .get("nodes")
        .or_else(|| session.get("controls"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let source_by_id = nodes_by_id(&source_nodes);
    let next_by_id = nodes_by_id(nodes);
    let source_ids = source_by_id.keys().cloned().collect::<BTreeSet<_>>();
    let next_ids = next_by_id.keys().cloned().collect::<BTreeSet<_>>();
    let added = next_ids
        .difference(&source_ids)
        .cloned()
        .collect::<Vec<_>>();
    let deleted = source_ids
        .difference(&next_ids)
        .cloned()
        .collect::<Vec<_>>();
    let mut moved = Vec::new();
    let mut resized = Vec::new();
    let mut reparented = Vec::new();
    let mut z_order_changed = Vec::new();
    let mut classification_changed = Vec::new();
    for id in source_ids.intersection(&next_ids) {
        let source = source_by_id[id];
        let next = next_by_id[id];
        let source_bounds = source.get("bounds").unwrap_or(&Value::Null);
        let next_bounds = next.get("bounds").unwrap_or(&Value::Null);
        if source_bounds.get("x") != next_bounds.get("x")
            || source_bounds.get("y") != next_bounds.get("y")
        {
            moved.push(id.clone());
        }
        if source_bounds.get("width") != next_bounds.get("width")
            || source_bounds.get("height") != next_bounds.get("height")
        {
            resized.push(id.clone());
        }
        if source.get("parent_id") != next.get("parent_id") {
            reparented.push(id.clone());
        }
        if source.get("z_index") != next.get("z_index") {
            z_order_changed.push(id.clone());
        }
        if source.get("node_kind") != next.get("node_kind")
            || source.get("render_mode") != next.get("render_mode")
        {
            classification_changed.push(id.clone());
        }
    }
    let changed_ids = added
        .iter()
        .chain(deleted.iter())
        .chain(moved.iter())
        .chain(resized.iter())
        .chain(reparented.iter())
        .chain(z_order_changed.iter())
        .chain(classification_changed.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    LayoutChangeSummary {
        changed_node_count: changed_ids.len(),
        added,
        deleted,
        moved,
        resized,
        reparented,
        z_order_changed,
        classification_changed,
    }
}

fn nodes_by_id(nodes: &[Value]) -> HashMap<String, &Value> {
    nodes
        .iter()
        .filter_map(|node| {
            node.get("id")
                .and_then(Value::as_str)
                .map(|id| (id.into(), node))
        })
        .collect()
}

fn write_snapshot(path: &Path, json: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("layout review path has no parent: {}", path.display()))?;
    let tmp = parent.join(".layout-review.json.tmp");
    {
        let mut file = fs::File::create(&tmp)
            .map_err(|error| format!("could not create temporary layout review: {error}"))?;
        file.write_all(json)
            .map_err(|error| format!("could not write temporary layout review: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("could not sync temporary layout review: {error}"))?;
    }
    atomic_replace(&tmp, path)
}

#[cfg(windows)]
fn atomic_replace(tmp: &Path, path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    if !path.exists() {
        return fs::rename(tmp, path)
            .map_err(|error| format!("could not install layout review: {error}"));
    }
    let target = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let replacement = tmp
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let replaced = unsafe {
        ReplaceFileW(
            target.as_ptr(),
            replacement.as_ptr(),
            ptr::null(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if replaced == 0 {
        let error = std::io::Error::last_os_error();
        let _ = fs::remove_file(tmp);
        return Err(format!(
            "could not atomically replace layout review: {error}"
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn atomic_replace(tmp: &Path, path: &Path) -> Result<(), String> {
    fs::rename(tmp, path).map_err(|error| format!("could not install layout review: {error}"))
}

fn format_saved_at(now_unix_ms: u64) -> Result<String, String> {
    let nanos = i128::from(now_unix_ms) * 1_000_000;
    OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .map_err(|error| format!("invalid layout review timestamp: {error}"))?
        .format(&Rfc3339)
        .map_err(|error| format!("could not format layout review timestamp: {error}"))
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_workbench_catalog::{WorkbenchCatalog, WorkbenchPage};
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("oasis-layout-review-{name}-{nonce}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn catalog_with_session(name: &str) -> (WorkbenchCatalog, PathBuf) {
        let session_dir = temp_dir(name);
        fs::write(
            session_dir.join("session.json"),
            serde_json::to_vec_pretty(&json!({
                "page_id": "resource-exchange",
                "title": "资源兑换",
                "source_image": "source.png",
                "page_size": { "width": 1415, "height": 794 },
                "nodes": [
                    {
                        "id": "panel.main",
                        "category": "panel",
                        "bounds": { "x": 10, "y": 20, "width": 600, "height": 400 },
                        "extraction": { "mode": "composite", "target_component_id": "panel.main" },
                        "z_index": 0
                    }
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let catalog = WorkbenchCatalog {
            schema_version: 1,
            selected_page_id: Some("resource-exchange".into()),
            pages: vec![WorkbenchPage {
                page_id: "resource-exchange".into(),
                title: "资源兑换".into(),
                session_dir: session_dir.clone(),
                source_image: "source.png".into(),
                thumbnail_image: None,
                control_count: 1,
                updated_at_unix_ms: 1,
            }],
        };
        (catalog, session_dir)
    }

    fn request(x: f64) -> LayoutReviewRequest {
        LayoutReviewRequest {
            workflow_task_id: Some("task-resource-exchange".into()),
            page_size: LayoutPageSize {
                width: 1415.0,
                height: 794.0,
            },
            nodes: vec![json!({
                "id": "panel.main",
                "category": "panel",
                "bounds": { "x": x, "y": 20, "width": 600, "height": 400 },
                "extraction": { "mode": "composite", "target_component_id": "panel.main" },
                "z_index": 0,
                "node_kind": "composite",
                "render_mode": "outline",
                "visible": true,
                "locked": false,
                "opacity": 1
            })],
        }
    }

    #[test]
    fn first_save_creates_revision_one_without_changing_session_json() {
        let (catalog, session_dir) = catalog_with_session("first-save");
        let session_before = fs::read(session_dir.join("session.json")).unwrap();

        let result = save_layout_review(
            &catalog,
            "resource-exchange",
            request(30.0),
            1_776_666_096_789,
        )
        .unwrap();
        let review = load_layout_review(&catalog, "resource-exchange")
            .unwrap()
            .unwrap();

        assert_eq!(result.revision, 1);
        assert_eq!(result.status, "pending_chat_confirmation");
        assert_eq!(review.revision, 1);
        assert_eq!(review.page_id, "resource-exchange");
        assert_eq!(
            review.source.workflow_task_id.as_deref(),
            Some("task-resource-exchange")
        );
        assert_eq!(review.change_summary.moved, vec!["panel.main"]);
        assert_eq!(
            fs::read(session_dir.join("session.json")).unwrap(),
            session_before
        );
    }

    #[test]
    fn repeated_save_increments_revision_and_replaces_the_snapshot() {
        let (catalog, session_dir) = catalog_with_session("repeat-save");

        save_layout_review(&catalog, "resource-exchange", request(30.0), 10).unwrap();
        let second = save_layout_review(&catalog, "resource-exchange", request(45.0), 20).unwrap();
        let raw = fs::read_to_string(session_dir.join("layout-review.json")).unwrap();
        let review: LayoutReview = serde_json::from_str(&raw).unwrap();

        assert_eq!(second.revision, 2);
        assert_eq!(review.revision, 2);
        assert_eq!(review.nodes[0]["bounds"]["x"], 45.0);
    }

    #[test]
    fn rejects_duplicate_missing_and_cyclic_parent_relationships() {
        let (catalog, _) = catalog_with_session("hierarchy-validation");
        let mut duplicate = request(30.0);
        duplicate.nodes.push(duplicate.nodes[0].clone());
        assert!(
            save_layout_review(&catalog, "resource-exchange", duplicate, 10)
                .unwrap_err()
                .contains("duplicate node id: panel.main")
        );

        let mut missing_parent = request(30.0);
        missing_parent.nodes[0]["parent_id"] = json!("panel.missing");
        assert!(
            save_layout_review(&catalog, "resource-exchange", missing_parent, 10)
                .unwrap_err()
                .contains("missing parent panel.missing")
        );

        let mut self_parent = request(30.0);
        self_parent.nodes[0]["parent_id"] = json!("panel.main");
        assert!(
            save_layout_review(&catalog, "resource-exchange", self_parent, 10)
                .unwrap_err()
                .contains("cannot parent itself")
        );

        let mut cycle = request(30.0);
        cycle.nodes[0]["parent_id"] = json!("panel.child");
        cycle.nodes.push(json!({
            "id": "panel.child",
            "category": "panel",
            "parent_id": "panel.main",
            "bounds": { "x": 20, "y": 30, "width": 100, "height": 80 },
            "extraction": { "mode": "composite", "target_component_id": "panel.child" },
            "z_index": 1,
            "node_kind": "composite",
            "render_mode": "outline"
        }));
        assert!(save_layout_review(&catalog, "resource-exchange", cycle, 10)
            .unwrap_err()
            .contains("parent cycle"));
    }

    #[test]
    fn rejects_invalid_node_fields_and_visual_asset_paths() {
        let (catalog, _) = catalog_with_session("field-validation");
        let invalid_cases = [
            (
                "category",
                Value::String(String::new()),
                "non-empty category",
            ),
            (
                "bounds.width",
                json!(0),
                "width must be finite and greater than zero",
            ),
            (
                "z_index",
                json!(1.5),
                "z_index must be a signed 32-bit integer",
            ),
            ("node_kind", json!("unknown"), "unsupported node_kind"),
            ("render_mode", json!("unknown"), "unsupported render_mode"),
        ];

        for (field, value, expected) in invalid_cases {
            let mut invalid = request(30.0);
            match field {
                "bounds.width" => invalid.nodes[0]["bounds"]["width"] = value,
                _ => invalid.nodes[0][field] = value,
            }
            let error = save_layout_review(&catalog, "resource-exchange", invalid, 10).unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }

        let mut traversal = request(30.0);
        traversal.nodes[0]["visual_assets"] = json!({
            "source_crop": "../outside.png",
            "clean_layer": null,
            "assembly_preview": "layers/panel.png"
        });
        assert!(
            save_layout_review(&catalog, "resource-exchange", traversal, 10)
                .unwrap_err()
                .contains("invalid visual asset path")
        );
    }

    #[test]
    fn rejected_save_preserves_the_previous_snapshot_bytes() {
        let (catalog, session_dir) = catalog_with_session("preserve-valid");
        save_layout_review(&catalog, "resource-exchange", request(30.0), 10).unwrap();
        let path = session_dir.join("layout-review.json");
        let before = fs::read(&path).unwrap();
        let mut invalid = request(45.0);
        invalid.nodes[0]["bounds"]["height"] = json!(-1);

        assert!(save_layout_review(&catalog, "resource-exchange", invalid, 20).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(!session_dir.join(".layout-review.json.tmp").exists());
    }

    #[test]
    fn malformed_snapshot_is_retained_and_recovery_restarts_at_revision_one() {
        let (catalog, session_dir) = catalog_with_session("malformed-recovery");
        fs::write(session_dir.join("layout-review.json"), b"{broken").unwrap();

        let result = save_layout_review(&catalog, "resource-exchange", request(30.0), 42).unwrap();
        let invalid_files = fs::read_dir(&session_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("layout-review.invalid-42")
            })
            .collect::<Vec<_>>();

        assert_eq!(result.revision, 1);
        assert_eq!(invalid_files.len(), 1);
        assert_eq!(fs::read(invalid_files[0].path()).unwrap(), b"{broken");
        assert!(load_layout_review(&catalog, "resource-exchange")
            .unwrap()
            .is_some());
    }

    #[test]
    fn loading_absent_and_unsupported_snapshots_is_explicit() {
        let (catalog, session_dir) = catalog_with_session("load-contract");
        assert_eq!(
            load_layout_review(&catalog, "resource-exchange").unwrap(),
            None
        );
        fs::write(
            session_dir.join("layout-review.json"),
            serde_json::to_vec(&json!({
                "artifact_type": "ui_layout_review",
                "schema_version": 2,
                "status": "pending_chat_confirmation",
                "page_id": "resource-exchange",
                "revision": 1,
                "saved_at": "2026-08-18T00:00:00Z",
                "source": { "session_file": "session.json", "session_sha256": "abc" },
                "page_size": { "width": 1, "height": 1 },
                "nodes": [],
                "change_summary": {
                    "changed_node_count": 0,
                    "added": [], "deleted": [], "moved": [], "resized": [],
                    "reparented": [], "z_order_changed": [], "classification_changed": []
                }
            }))
            .unwrap(),
        )
        .unwrap();

        assert!(load_layout_review(&catalog, "resource-exchange")
            .unwrap_err()
            .contains("unsupported layout review schema"));
    }
}
