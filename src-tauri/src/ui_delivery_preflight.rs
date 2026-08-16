use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::AppHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSearchSpec {
    pub root: String,
    pub filter: Option<String>,
    pub exact_load_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WidgetBlueprintCandidate {
    pub display_name: String,
    pub load_path: String,
    pub class_name: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryPreflightState {
    Idle,
    CheckingMcp,
    SearchingAssets,
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryPreflightEvidence {
    pub status: DeliveryPreflightState,
    pub checked_at_unix_ms: u64,
    pub mcp_server_name: String,
    pub mcp_server_version: String,
    pub editor_project_root: String,
    pub selected_load_path: String,
    pub selected_class_name: String,
    pub evidence_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WidgetBlueprintSearchResult {
    pub state: DeliveryPreflightState,
    pub candidates: Vec<WidgetBlueprintCandidate>,
    pub message: String,
}

pub fn validate_project_workspace(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value.trim());
    if !path.is_absolute() {
        return Err("project workspace must be absolute".into());
    }
    if !path.is_dir() {
        return Err("project workspace is unavailable".into());
    }
    Ok(path)
}

pub fn validate_search_query(project_root: &str, query: &str) -> Result<AssetSearchSpec, String> {
    let project_root = normalize_project_root(project_root)?;
    let query = query.trim().replace('\\', "/");
    if query.is_empty() {
        return Err("WidgetBlueprint search is required".into());
    }
    if query.len() > 512 || query.chars().any(char::is_control) {
        return Err("WidgetBlueprint search is invalid".into());
    }
    if query.contains("..") || query.starts_with("/Game/") || query == "/Game" {
        return Err("WidgetBlueprint search must stay inside the editor project".into());
    }

    if query.starts_with('/') {
        let asset_root = format!("{project_root}/Asset");
        if query != asset_root && !query.starts_with(&(asset_root.clone() + "/")) {
            return Err("WidgetBlueprint search is outside the editor project".into());
        }
        let leaf = query.rsplit('/').next().unwrap_or_default();
        if leaf.contains('.') {
            let parent = query
                .rsplit_once('/')
                .map(|(parent, _)| parent)
                .ok_or_else(|| {
                    "WidgetBlueprint object path has no package directory".to_string()
                })?;
            return Ok(AssetSearchSpec {
                root: parent.to_owned(),
                filter: Some(query.to_ascii_lowercase()),
                exact_load_path: Some(query),
            });
        }
        return Ok(AssetSearchSpec {
            root: query,
            filter: None,
            exact_load_path: None,
        });
    }

    if query.chars().count() < 2 {
        return Err("WidgetBlueprint text search must contain at least two characters".into());
    }
    Ok(AssetSearchSpec {
        root: format!("{project_root}/Asset/UI"),
        filter: Some(query.to_ascii_lowercase()),
        exact_load_path: None,
    })
}

pub fn parse_editor_project_root(content: &Value) -> Result<String, String> {
    let decoded = decode_mcp_content(content)?;
    if let Some(error) = find_editor_error(&decoded) {
        return Err(error);
    }
    find_string_field(&decoded, &["project_root", "project_mount", "root"])
        .ok_or_else(|| "editor context did not report a project root".to_string())
        .and_then(|root| normalize_project_root(&root))
}

pub fn parse_widget_blueprint_candidates(
    content: &Value,
) -> Result<Vec<WidgetBlueprintCandidate>, String> {
    let decoded = decode_mcp_content(content)?;
    if find_bool_field(&decoded, "truncated") == Some(true) {
        return Err("asset search was truncated; narrow the query".into());
    }
    let items = find_array_field(&decoded, "matches")
        .or_else(|| decoded.as_array())
        .ok_or_else(|| "asset search did not return a candidate list".to_string())?;
    let mut candidates = Vec::new();
    for item in items {
        let Some(object) = item.as_object() else {
            continue;
        };
        let class_name = object
            .get("class_name")
            .or_else(|| object.get("asset_class"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(class_name, "UGCWidgetBlueprint" | "WidgetBlueprint") {
            continue;
        }
        let load_path = object
            .get("load_path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        if !is_widget_blueprint_load_path(load_path) {
            continue;
        }
        let display_name = object
            .get("display_name")
            .or_else(|| object.get("name"))
            .or_else(|| object.get("asset_name"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        if display_name.is_empty() {
            continue;
        }
        candidates.push(WidgetBlueprintCandidate {
            display_name: display_name.to_owned(),
            load_path: load_path.to_owned(),
            class_name: class_name.to_owned(),
        });
    }
    candidates.sort_by(|left, right| {
        left.display_name
            .to_ascii_lowercase()
            .cmp(&right.display_name.to_ascii_lowercase())
            .then_with(|| left.load_path.cmp(&right.load_path))
    });
    if candidates.len() > 100 {
        return Err(
            "asset search returned more than 100 WidgetBlueprints; narrow the query".into(),
        );
    }
    Ok(candidates)
}

pub fn select_exact_candidate(
    candidates: &[WidgetBlueprintCandidate],
    load_path: &str,
) -> Result<WidgetBlueprintCandidate, String> {
    let matches = candidates
        .iter()
        .filter(|candidate| candidate.load_path == load_path)
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [candidate] => Ok(candidate.clone()),
        [] => Err("selected WidgetBlueprint was not returned by the editor".into()),
        _ => Err("editor returned more than one exact WidgetBlueprint match".into()),
    }
}

fn read_only_server_identity(status: &crate::mcp::McpStatus) -> Result<(String, String), String> {
    if !status.enabled {
        return Err("编辑器 MCP 未启用".into());
    }
    for required in ["ue_read", "ue_py"] {
        if !status.tools.iter().any(|tool| tool.name == required) {
            return Err(format!("编辑器 MCP 缺少 {required}"));
        }
    }
    let server = status.server_info.as_ref().ok_or_else(|| {
        status
            .error
            .clone()
            .unwrap_or_else(|| "编辑器 MCP 未返回服务器信息".into())
    })?;
    Ok((server.name.clone(), server.version.clone()))
}

fn build_asset_query_code(search_root: &str, filter: Option<&str>) -> Result<String, String> {
    let root_literal = serde_json::to_string(search_root).map_err(|error| error.to_string())?;
    let filter_literal =
        serde_json::to_string(filter.unwrap_or_default()).map_err(|error| error.to_string())?;
    Ok(format!(
        r#"import unreal_engine as ue
search_root = {root_literal}
filter_text = {filter_literal}.lower()
items = ue.list_assets(search_root, True)
matches = []
for item in items:
    name = item.get('name') or item.get('asset_name') or ''
    asset_class = item.get('asset_class') or item.get('class_name') or ''
    load_path = item.get('load_path') or ''
    if asset_class not in ('UGCWidgetBlueprint', 'WidgetBlueprint'):
        continue
    if filter_text and filter_text not in name.lower() and filter_text not in load_path.lower():
        continue
    matches.append({{
        'name': name,
        'asset_class': asset_class,
        'load_path': load_path,
    }})
    if len(matches) > 100:
        break
__askq_result = {{'project_root': ue.get_project_root(), 'matches': matches[:100], 'truncated': len(matches) > 100}}"#
    ))
}

pub async fn search_widget_blueprints(
    app: &AppHandle,
    project_workspace: &str,
    query: &str,
) -> Result<(WidgetBlueprintSearchResult, PreflightContext), String> {
    let workspace = validate_project_workspace(project_workspace)?;
    let status = crate::mcp::check(app).await;
    let (mcp_server_name, mcp_server_version) = read_only_server_identity(&status)?;
    let discovery = crate::mcp::call_tool(
        app,
        "ue_read",
        serde_json::json!({"queries": ["ctx:", "py:workflow asset_browser"]}),
    )
    .await?;
    if discovery.is_error {
        return Err("编辑器上下文读取失败".into());
    }

    let workspace_name = workspace
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "项目工作区名称不可用".to_string())?;
    let provisional_root = format!("/{workspace_name}");
    let spec = validate_search_query(&provisional_root, query)?;
    let code = build_asset_query_code(&spec.root, spec.filter.as_deref())?;
    let result = crate::mcp::call_tool(
        app,
        "ue_py",
        serde_json::json!({
            "instruction": "List WidgetBlueprint candidates for Oasis UI delivery preflight without changing editor state.",
            "transaction_name": "Oasis UI Delivery Preflight",
            "code": code,
        }),
    )
    .await?;
    if result.is_error {
        return Err("编辑器资产查询失败".into());
    }
    let editor_project_root = parse_editor_project_root(&result.content)?;
    if editor_project_root.trim_start_matches('/') != workspace_name {
        return Err(format!(
            "当前编辑器项目 {editor_project_root} 与工作区 {workspace_name} 不匹配"
        ));
    }
    let mut candidates = parse_widget_blueprint_candidates(&result.content)?;
    if let Some(filter) = &spec.filter {
        candidates.retain(|candidate| {
            candidate.display_name.to_ascii_lowercase().contains(filter)
                || candidate.load_path.to_ascii_lowercase().contains(filter)
        });
    }
    Ok((
        WidgetBlueprintSearchResult {
            state: DeliveryPreflightState::Ready,
            candidates,
            message: "已读取编辑器 WidgetBlueprint".into(),
        },
        PreflightContext {
            editor_project_root,
            mcp_server_name,
            mcp_server_version,
        },
    ))
}

pub async fn validate_exact_widget_blueprint(
    app: &AppHandle,
    task_id: &str,
    project_workspace: &str,
    selected_load_path: &str,
    checked_at_unix_ms: u64,
) -> Result<(WidgetBlueprintCandidate, DeliveryPreflightEvidence), String> {
    let (search, context) =
        search_widget_blueprints(app, project_workspace, selected_load_path).await?;
    let candidate = select_exact_candidate(&search.candidates, selected_load_path)?;
    let mut hasher = Sha256::new();
    for value in [
        task_id,
        project_workspace,
        &context.editor_project_root,
        &candidate.load_path,
        &candidate.class_name,
        &context.mcp_server_name,
        &context.mcp_server_version,
        &checked_at_unix_ms.to_string(),
    ] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    let evidence = DeliveryPreflightEvidence {
        status: DeliveryPreflightState::Ready,
        checked_at_unix_ms,
        mcp_server_name: context.mcp_server_name,
        mcp_server_version: context.mcp_server_version,
        editor_project_root: context.editor_project_root,
        selected_load_path: candidate.load_path.clone(),
        selected_class_name: candidate.class_name.clone(),
        evidence_id: format!("sha256:{:x}", hasher.finalize()),
        message: "目标 WidgetBlueprint 已通过编辑器只读预检".into(),
    };
    Ok((candidate, evidence))
}

#[derive(Debug, Clone)]
pub struct PreflightContext {
    pub editor_project_root: String,
    pub mcp_server_name: String,
    pub mcp_server_version: String,
}

fn normalize_project_root(value: &str) -> Result<String, String> {
    let root = value.trim().trim_end_matches('/');
    if !root.starts_with('/')
        || root == "/Game"
        || root.contains("/Asset")
        || root.contains("..")
        || root[1..].contains('/')
    {
        return Err("editor project root is invalid".into());
    }
    Ok(root.to_owned())
}

fn is_widget_blueprint_load_path(value: &str) -> bool {
    value.starts_with('/')
        && value.contains("/Asset/")
        && value.rsplit('/').next().is_some_and(|leaf| {
            leaf.split_once('.').is_some_and(|(package, object)| {
                !package.is_empty() && !object.is_empty() && !object.ends_with("_C")
            })
        })
}

fn decode_mcp_content(content: &Value) -> Result<Value, String> {
    if let Some(text) = content.as_str() {
        return serde_json::from_str(text)
            .map_err(|error| format!("MCP text result is not valid JSON: {error}"));
    }
    if let Some(array) = content.as_array() {
        for item in array {
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                if let Ok(decoded) = serde_json::from_str(text) {
                    return Ok(decoded);
                }
            }
        }
    }
    Ok(content.clone())
}

fn find_string_field(value: &Value, names: &[&str]) -> Option<String> {
    match value {
        Value::Object(object) => {
            for name in names {
                if let Some(result) = object.get(*name).and_then(Value::as_str) {
                    return Some(result.to_owned());
                }
            }
            object
                .values()
                .find_map(|child| find_string_field(child, names))
        }
        Value::Array(array) => array
            .iter()
            .find_map(|child| find_string_field(child, names)),
        _ => None,
    }
}

fn find_editor_error(value: &Value) -> Option<String> {
    match value {
        Value::Object(object) => {
            if object.get("success").and_then(Value::as_bool) == Some(false) {
                if let Some(error) = object.get("error").and_then(Value::as_object) {
                    let message = error.get("message").and_then(Value::as_str)?;
                    let error_type = error.get("type").and_then(Value::as_str).unwrap_or("Error");
                    return Some(format!("编辑器 {error_type}: {message}"));
                }
            }
            object.values().find_map(find_editor_error)
        }
        Value::Array(array) => array.iter().find_map(find_editor_error),
        _ => None,
    }
}

fn find_bool_field(value: &Value, name: &str) -> Option<bool> {
    match value {
        Value::Object(object) => object.get(name).and_then(Value::as_bool).or_else(|| {
            object
                .values()
                .find_map(|child| find_bool_field(child, name))
        }),
        Value::Array(array) => array.iter().find_map(|child| find_bool_field(child, name)),
        _ => None,
    }
}

fn find_array_field<'a>(value: &'a Value, name: &str) -> Option<&'a Vec<Value>> {
    match value {
        Value::Object(object) => object.get(name).and_then(Value::as_array).or_else(|| {
            object
                .values()
                .find_map(|child| find_array_field(child, name))
        }),
        Value::Array(array) => array.iter().find_map(|child| find_array_field(child, name)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_workspace() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("oasis-preflight-{stamp}"));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn validates_existing_absolute_project_workspaces() {
        let workspace = temp_workspace();
        assert_eq!(
            validate_project_workspace(workspace.to_str().unwrap()).unwrap(),
            workspace
        );
        fs::remove_dir_all(workspace).unwrap();
        assert!(validate_project_workspace("relative/project").is_err());
        assert!(validate_project_workspace(r"C:\missing-oasis-project").is_err());
    }

    #[test]
    fn resolves_exact_directory_and_text_searches_inside_the_editor_project() {
        assert_eq!(
            validate_search_query("/RedCliff", "/RedCliff/Asset/UI/Currency.Currency").unwrap(),
            AssetSearchSpec {
                root: "/RedCliff/Asset/UI".into(),
                filter: Some("/redcliff/asset/ui/currency.currency".into()),
                exact_load_path: Some("/RedCliff/Asset/UI/Currency.Currency".into()),
            }
        );
        assert_eq!(
            validate_search_query("/RedCliff", "/RedCliff/Asset/UI").unwrap(),
            AssetSearchSpec {
                root: "/RedCliff/Asset/UI".into(),
                filter: None,
                exact_load_path: None,
            }
        );
        assert_eq!(
            validate_search_query("/RedCliff", "currency").unwrap(),
            AssetSearchSpec {
                root: "/RedCliff/Asset/UI".into(),
                filter: Some("currency".into()),
                exact_load_path: None,
            }
        );
        for query in ["", "x", "/Game/UI", "/Other/Asset/UI", "../UI"] {
            assert!(
                validate_search_query("/RedCliff", query).is_err(),
                "{query}"
            );
        }
    }

    #[test]
    fn parses_editor_context_and_supported_widget_blueprints() {
        let context = json!([{
            "type": "text",
            "text": "{\"results\":[{\"success\":true,\"output\":{\"project_root\":\"/RedCliff\"}}]}"
        }]);
        assert_eq!(parse_editor_project_root(&context).unwrap(), "/RedCliff");

        let assets = json!([{
            "type": "text",
            "text": "{\"matches\":[\
              {\"name\":\"Currency\",\"asset_class\":\"UGCWidgetBlueprint\",\"load_path\":\"/RedCliff/Asset/UI/Currency.Currency\"},\
              {\"name\":\"Shop\",\"asset_class\":\"WidgetBlueprint\",\"load_path\":\"/RedCliff/Asset/UI/Shop.Shop\"},\
              {\"name\":\"Generated\",\"asset_class\":\"WidgetBlueprintGeneratedClass\",\"load_path\":\"/RedCliff/Asset/UI/Generated.Generated_C\"},\
              {\"name\":\"Ordinary\",\"asset_class\":\"Blueprint\",\"load_path\":\"/RedCliff/Asset/UI/Ordinary.Ordinary\"}\
            ],\"truncated\":false}"
        }]);
        assert_eq!(
            parse_widget_blueprint_candidates(&assets).unwrap(),
            vec![
                WidgetBlueprintCandidate {
                    display_name: "Currency".into(),
                    load_path: "/RedCliff/Asset/UI/Currency.Currency".into(),
                    class_name: "UGCWidgetBlueprint".into(),
                },
                WidgetBlueprintCandidate {
                    display_name: "Shop".into(),
                    load_path: "/RedCliff/Asset/UI/Shop.Shop".into(),
                    class_name: "WidgetBlueprint".into(),
                },
            ]
        );
    }

    #[test]
    fn selects_one_exact_editor_returned_candidate() {
        let candidates = vec![
            WidgetBlueprintCandidate {
                display_name: "Currency".into(),
                load_path: "/RedCliff/Asset/UI/Currency.Currency".into(),
                class_name: "UGCWidgetBlueprint".into(),
            },
            WidgetBlueprintCandidate {
                display_name: "Shop".into(),
                load_path: "/RedCliff/Asset/UI/Shop.Shop".into(),
                class_name: "WidgetBlueprint".into(),
            },
        ];
        assert_eq!(
            select_exact_candidate(&candidates, "/RedCliff/Asset/UI/Currency.Currency").unwrap(),
            candidates[0]
        );
        assert!(select_exact_candidate(&candidates, "/RedCliff/Asset/UI/Missing.Missing").is_err());

        let duplicates = vec![candidates[0].clone(), candidates[0].clone()];
        assert!(
            select_exact_candidate(&duplicates, "/RedCliff/Asset/UI/Currency.Currency").is_err()
        );
    }

    #[test]
    fn read_only_preflight_requires_only_read_and_python_tools() {
        let status = crate::mcp::McpStatus {
            enabled: true,
            state: crate::mcp::McpConnectionState::Disconnected,
            url: "http://127.0.0.1:12463/sse".into(),
            server_info: Some(crate::mcp::ServerInfo {
                name: "UGCAskQ".into(),
                version: "1.0.0".into(),
            }),
            tools: vec![
                crate::config::McpToolInfo {
                    name: "ue_read".into(),
                    description: String::new(),
                },
                crate::config::McpToolInfo {
                    name: "ue_py".into(),
                    description: String::new(),
                },
            ],
            checked_at: "1".into(),
            error: Some("write tool unavailable".into()),
        };

        assert_eq!(
            read_only_server_identity(&status).unwrap(),
            ("UGCAskQ".into(), "1.0.0".into())
        );
    }

    #[test]
    fn generated_asset_query_filters_and_bounds_editor_results_without_writes() {
        let code = build_asset_query_code("/RedCliff/Asset/UI", Some("currency")).unwrap();

        assert!(code.contains("ue.list_assets"));
        assert!(code.contains("filter_text = \"currency\".lower()"));
        assert!(code.contains("asset_class not in ('UGCWidgetBlueprint', 'WidgetBlueprint')"));
        assert!(code.contains("if len(matches) > 100:"));
        assert!(code.contains("'matches': matches[:100]"));
        assert!(code.contains("'truncated': len(matches) > 100"));
        for forbidden in ["ue_plan_submit", "skip_prv", "plan_id", "save_asset"] {
            assert!(!code.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn generated_asset_query_preserves_python_block_indentation() {
        let code = build_asset_query_code("/RedCliff/Asset/UI", Some("currency")).unwrap();

        assert!(code.contains("for item in items:\n    name ="));
        assert!(code.contains(
            "if asset_class not in ('UGCWidgetBlueprint', 'WidgetBlueprint'):\n        continue"
        ));
        assert!(code.contains("if len(matches) > 100:\n        break"));
    }

    #[test]
    fn editor_python_errors_are_reported_before_project_root_validation() {
        let content = json!([{
            "type": "text",
            "text": "{\"success\":false,\"result\":null,\"error\":{\"message\":\"expected an indented block\",\"type\":\"PythonError\"}}"
        }]);

        let error = parse_editor_project_root(&content).unwrap_err();

        assert_eq!(error, "编辑器 PythonError: expected an indented block");
    }
}
