//! Persistent UI Workbench page catalog.

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CATALOG_SCHEMA_VERSION: u32 = 1;

/// One generated UI page registered with Oasis Companion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkbenchPage {
    pub page_id: String,
    pub title: String,
    pub session_dir: PathBuf,
    pub source_image: String,
    pub thumbnail_image: Option<String>,
    pub control_count: usize,
    pub updated_at_unix_ms: u64,
}

/// Durable list of generated UI pages and the last selected page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkbenchCatalog {
    pub schema_version: u32,
    pub selected_page_id: Option<String>,
    pub pages: Vec<WorkbenchPage>,
}

impl Default for WorkbenchCatalog {
    fn default() -> Self {
        Self {
            schema_version: CATALOG_SCHEMA_VERSION,
            selected_page_id: None,
            pages: Vec::new(),
        }
    }
}

/// A persisted page paired with its parsed `session.json`.
#[derive(Debug, Clone, Serialize)]
pub struct LoadedWorkbenchPage {
    pub page: WorkbenchPage,
    pub session: Value,
}

/// Frontend-safe metadata for one registered page.
#[derive(Debug, Clone, Serialize)]
pub struct WorkbenchPageSummary {
    pub page_id: String,
    pub title: String,
    pub control_count: usize,
    pub updated_at_unix_ms: u64,
    pub thumbnail_data_url: Option<String>,
    pub available: bool,
}

/// Frontend-safe catalog state.
#[derive(Debug, Clone, Serialize)]
pub struct WorkbenchCatalogView {
    pub selected_page_id: Option<String>,
    pub pages: Vec<WorkbenchPageSummary>,
}

/// Frontend-safe persisted session data.
#[derive(Debug, Clone, Serialize)]
pub struct LoadedWorkbenchPageView {
    pub page_id: String,
    pub title: String,
    pub control_count: usize,
    pub source_image: String,
    pub session: Value,
}

/// Return the user-level catalog path.
pub fn catalog_path() -> PathBuf {
    crate::config::settings_dir().join("ui-workbench-pages.json")
}

/// Load the user-level page catalog, recovering invalid JSON to an empty catalog.
pub fn load_catalog() -> WorkbenchCatalog {
    let path = catalog_path();
    match load_catalog_from_path(&path) {
        Ok(catalog) => catalog,
        Err(error) => {
            log::warn!("UI Workbench catalog could not be loaded: {error}");
            if path.exists() {
                let stamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0);
                let backup = path.with_extension(format!("corrupt.{stamp}.json"));
                if let Err(rename_error) = fs::rename(&path, &backup) {
                    log::warn!("UI Workbench catalog backup failed: {rename_error}");
                }
            }
            WorkbenchCatalog::default()
        }
    }
}

/// Load a catalog from an explicit path.
pub fn load_catalog_from_path(path: &Path) -> Result<WorkbenchCatalog, String> {
    if !path.exists() {
        return Ok(WorkbenchCatalog::default());
    }
    let raw =
        fs::read_to_string(path).map_err(|error| format!("could not read catalog: {error}"))?;
    let catalog: WorkbenchCatalog =
        serde_json::from_str(&raw).map_err(|error| format!("invalid catalog JSON: {error}"))?;
    if catalog.schema_version != CATALOG_SCHEMA_VERSION {
        return Err(format!(
            "unsupported catalog schema version: {}",
            catalog.schema_version
        ));
    }
    Ok(catalog)
}

/// Atomically persist a catalog at the user-level path.
pub fn save_catalog(catalog: &WorkbenchCatalog) -> Result<(), String> {
    save_catalog_to_path(catalog, &catalog_path())
}

/// Atomically persist a catalog at an explicit path.
pub fn save_catalog_to_path(catalog: &WorkbenchCatalog, path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("catalog path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create catalog directory: {error}"))?;
    let tmp = parent.join(".ui-workbench-pages.json.tmp");
    let json = serde_json::to_vec_pretty(catalog)
        .map_err(|error| format!("could not serialize catalog: {error}"))?;
    {
        let mut file = fs::File::create(&tmp)
            .map_err(|error| format!("could not create temporary catalog: {error}"))?;
        file.write_all(&json)
            .map_err(|error| format!("could not write temporary catalog: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("could not sync temporary catalog: {error}"))?;
    }
    if path.exists() {
        fs::remove_file(path).map_err(|error| format!("could not replace catalog: {error}"))?;
    }
    fs::rename(&tmp, path).map_err(|error| format!("could not install catalog: {error}"))
}

/// Register a generated session directory, replacing any page with the same ID.
pub fn register_session_dir(
    catalog: &mut WorkbenchCatalog,
    session_dir: &Path,
    updated_at_unix_ms: u64,
) -> Result<WorkbenchPage, String> {
    if !session_dir.is_absolute() {
        return Err("workbench session directory must be absolute".into());
    }
    let session_dir = session_dir
        .canonicalize()
        .map_err(|error| format!("workbench session directory is unavailable: {error}"))?;
    if !session_dir.is_dir() {
        return Err("workbench session path must be a directory".into());
    }
    let session_path = session_dir.join("session.json");
    if !session_path.is_file() {
        return Err("workbench session directory must contain session.json".into());
    }
    let session = read_session_json(&session_path)?;
    let page_id = required_session_string(&session, "page_id")?;
    if !page_id.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    }) {
        return Err("session page_id must use lowercase ASCII letters, digits, or hyphens".into());
    }
    let title = required_session_string(&session, "title")?;
    let source_image = required_session_string(&session, "source_image")?;
    resolve_asset_path(&session_dir, &source_image)?;
    let thumbnail_image = session
        .get("thumbnail_image")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned);
    if let Some(path) = &thumbnail_image {
        resolve_asset_path(&session_dir, path)?;
    }
    let control_count = session
        .get("controls")
        .or_else(|| session.get("nodes"))
        .and_then(Value::as_array)
        .ok_or_else(|| "session must contain a controls or nodes array".to_string())?
        .len();
    let page = WorkbenchPage {
        page_id: page_id.clone(),
        title,
        session_dir,
        source_image,
        thumbnail_image,
        control_count,
        updated_at_unix_ms,
    };
    catalog.pages.retain(|existing| existing.page_id != page_id);
    catalog.pages.insert(0, page.clone());
    catalog.selected_page_id = Some(page_id);
    Ok(page)
}

/// Mark one registered page as selected.
pub fn select_page(catalog: &mut WorkbenchCatalog, page_id: &str) -> Result<(), String> {
    if !catalog.pages.iter().any(|page| page.page_id == page_id) {
        return Err(format!("unknown UI Workbench page: {page_id}"));
    }
    catalog.selected_page_id = Some(page_id.to_owned());
    Ok(())
}

/// Load the current `session.json` for one registered page.
pub fn load_page(catalog: &WorkbenchCatalog, page_id: &str) -> Result<LoadedWorkbenchPage, String> {
    let page = find_page(catalog, page_id)?.clone();
    let session = read_session_json(&page.session_dir.join("session.json"))?;
    Ok(LoadedWorkbenchPage { page, session })
}

/// Read one registered relative asset as a browser-ready data URL.
pub fn read_page_asset(
    catalog: &WorkbenchCatalog,
    page_id: &str,
    asset_path: &str,
) -> Result<String, String> {
    let page = find_page(catalog, page_id)?;
    let path = resolve_asset_path(&page.session_dir, asset_path)?;
    let bytes =
        fs::read(&path).map_err(|error| format!("could not read workbench asset: {error}"))?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
}

/// Build navigation metadata without exposing local session paths.
pub fn catalog_view(catalog: &WorkbenchCatalog) -> WorkbenchCatalogView {
    let pages = catalog
        .pages
        .iter()
        .map(|page| {
            let available = page.session_dir.join("session.json").is_file();
            let thumbnail_data_url = page
                .thumbnail_image
                .as_deref()
                .and_then(|path| read_page_asset(catalog, &page.page_id, path).ok());
            WorkbenchPageSummary {
                page_id: page.page_id.clone(),
                title: page.title.clone(),
                control_count: page.control_count,
                updated_at_unix_ms: page.updated_at_unix_ms,
                thumbnail_data_url,
                available,
            }
        })
        .collect();
    WorkbenchCatalogView {
        selected_page_id: catalog.selected_page_id.clone(),
        pages,
    }
}

/// Load one persisted page without exposing its local directory.
pub fn load_page_view(
    catalog: &WorkbenchCatalog,
    page_id: &str,
) -> Result<LoadedWorkbenchPageView, String> {
    let loaded = load_page(catalog, page_id)?;
    Ok(LoadedWorkbenchPageView {
        page_id: loaded.page.page_id,
        title: loaded.page.title,
        control_count: loaded.page.control_count,
        source_image: loaded.page.source_image,
        session: loaded.session,
    })
}

fn find_page<'a>(
    catalog: &'a WorkbenchCatalog,
    page_id: &str,
) -> Result<&'a WorkbenchPage, String> {
    catalog
        .pages
        .iter()
        .find(|page| page.page_id == page_id)
        .ok_or_else(|| format!("unknown UI Workbench page: {page_id}"))
}

fn read_session_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let session: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    if !session.is_object() {
        return Err("session.json must contain an object".into());
    }
    Ok(session)
}

fn required_session_string(session: &Value, field: &str) -> Result<String, String> {
    session
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("session must contain a non-empty {field}"))
}

fn resolve_asset_path(session_dir: &Path, asset_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(asset_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "invalid relative workbench asset path: {asset_path}"
        ));
    }
    let path = session_dir.join(relative);
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("workbench asset is unavailable: {error}"))?;
    if !canonical.starts_with(session_dir) || !canonical.is_file() {
        return Err(format!(
            "workbench asset is outside the registered session: {asset_path}"
        ));
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("oasis-workbench-{name}-{nonce}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_session(dir: &Path, page_id: &str, title: &str, control_count: usize) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("source.png"), b"png-source").unwrap();
        fs::write(dir.join("thumbnail.webp"), b"webp-thumbnail").unwrap();
        fs::create_dir_all(dir.join("layers")).unwrap();
        fs::write(dir.join("layers/button.png"), b"png-layer").unwrap();
        let controls = (0..control_count)
            .map(|index| json!({ "component_id": format!("control.{index}") }))
            .collect::<Vec<_>>();
        fs::write(
            dir.join("session.json"),
            serde_json::to_vec_pretty(&json!({
                "schema_version": 3,
                "page_id": page_id,
                "title": title,
                "source_image": "source.png",
                "thumbnail_image": "thumbnail.webp",
                "controls": controls,
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn registration_replaces_the_same_page_id_and_persists_selection() {
        let root = temp_dir("replace");
        let first = root.join("first");
        let latest = root.join("latest");
        write_session(&first, "currency", "货币兑换", 3);
        write_session(&latest, "currency", "货币兑换新版", 5);
        let path = root.join("catalog.json");
        let mut catalog = WorkbenchCatalog::default();

        register_session_dir(&mut catalog, &first, 10).unwrap();
        register_session_dir(&mut catalog, &latest, 20).unwrap();
        save_catalog_to_path(&catalog, &path).unwrap();
        let loaded = load_catalog_from_path(&path).unwrap();

        assert_eq!(loaded.pages.len(), 1);
        assert_eq!(loaded.pages[0].title, "货币兑换新版");
        assert_eq!(loaded.pages[0].control_count, 5);
        assert_eq!(loaded.selected_page_id.as_deref(), Some("currency"));
        assert_eq!(loaded.pages[0].session_dir, latest.canonicalize().unwrap());
    }

    #[test]
    fn registered_assets_cannot_escape_the_session_directory() {
        let root = temp_dir("asset");
        let session = root.join("session");
        write_session(&session, "gem-draw", "宝石抽奖", 2);
        fs::write(root.join("outside.png"), b"secret").unwrap();
        let mut catalog = WorkbenchCatalog::default();
        register_session_dir(&mut catalog, &session, 10).unwrap();

        assert_eq!(
            read_page_asset(&catalog, "gem-draw", "layers/button.png").unwrap(),
            "data:image/png;base64,cG5nLWxheWVy"
        );
        assert!(read_page_asset(&catalog, "gem-draw", "../outside.png").is_err());
        assert!(read_page_asset(
            &catalog,
            "gem-draw",
            root.join("outside.png").to_str().unwrap()
        )
        .is_err());
    }

    #[test]
    fn missing_session_metadata_is_rejected_without_changing_the_catalog() {
        let root = temp_dir("invalid");
        let session = root.join("session");
        fs::create_dir_all(&session).unwrap();
        fs::write(session.join("session.json"), br#"{"title":"Missing ID"}"#).unwrap();
        let mut catalog = WorkbenchCatalog::default();

        assert!(register_session_dir(&mut catalog, &session, 10).is_err());
        assert!(catalog.pages.is_empty());
        assert_eq!(catalog.selected_page_id, None);
    }
}
