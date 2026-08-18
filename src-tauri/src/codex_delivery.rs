//! Delivery of an approved UI request to a new Codex task.

use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiSourceMode {
    Generate,
    Import,
}

impl UiSourceMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "generate" => Ok(Self::Generate),
            "import" => Ok(Self::Import),
            _ => Err(format!("unsupported UI source mode: {value}")),
        }
    }
}

pub fn bundled_reporter_path(resource_dir: &Path) -> PathBuf {
    resource_dir
        .join("resources")
        .join("skill")
        .join("scripts")
        .join("cowart-ui")
        .join("component-extractor")
        .join("report_ui_workflow_progress.py")
}

pub fn build_delivery_prompt(
    request_path: &Path,
    reporter_path: &Path,
    session_dir: &Path,
    task_id: &str,
) -> String {
    format!(
        "这是一个新的 Codex 任务，请开始实现已确认的 Oasis UI 交付。读取交付文件：{}\n\
必须调用 oasis-wiki Skill，先读取 delivery-request.json 中的冻结证据，并通过 MCP 重读其中的精确 load_path。\n\
如资产类型、路径、编辑器项目或证据发生目标漂移，立即上报 blocked 并停止，不得创建或猜测替代资产。\n\
只在冻结的 WidgetBlueprint 范围内执行；所有编辑器写入必须遵守 oasis-wiki PRV、备份和事务要求。\n\
执行当前项目读取、UGCAskQ MCP 写入、保存回读和编辑器可见验收。\n\
在 UMG、逻辑和验收阶段使用以下完整命令格式上报进度：\n\
python \"{}\" --session-dir \"{}\" --task-id \"{}\" --stage umg --status in_progress --message \"开始实现 WidgetBlueprint\"\n\
python \"{}\" --session-dir \"{}\" --task-id \"{}\" --stage logic --status completed --message \"逻辑绑定已保存并回读\"\n\
python \"{}\" --session-dir \"{}\" --task-id \"{}\" --stage review --status completed --message \"编辑器可见验收通过\"\n\
如需附带会话内证据文件，在命令末尾重复添加 --artifact <相对路径>。\n\
遇到目标、MCP 或授权不完整时上报 blocked 并停止，不要扩大修改范围。",
        request_path.display(),
        reporter_path.display(),
        session_dir.display(),
        task_id,
        reporter_path.display(),
        session_dir.display(),
        task_id,
        reporter_path.display(),
        session_dir.display(),
        task_id,
    )
}

pub fn build_ui_source_prompt(mode: UiSourceMode) -> String {
    let source_instruction = match mode {
        UiSourceMode::Generate => {
            "我要生成新的 UI。立即进入 SOURCE，读取当前项目的 Game UI Design System、可复用控件和项目风格，只询问一个真正缺失的信息，然后开始 UI Tree 与视觉稿流程。"
        }
        UiSourceMode::Import => {
            "我有一张已有 UI 图片需要导入。立即进入 SOURCE 的已有图分支，先让我提供图片或确认本机图片路径，再执行 UI Tree 推断、控件分类、分层和 Workbench 流程。"
        }
    };
    format!(
        "这是一个新的 Codex 任务，请调用 oasis-wiki Skill 启动 Oasis UI 生图工具链。\n\
{source_instruction}\n\
使用当前任务绑定的项目工作区，不要继续旧 UI 页面，也不要把旧页面的控件树、图片或资产混入新任务。\n\
保留 Companion 八阶段进度同步、原生文字控件、localhost 回退和所有现有授权门禁。\n\
未经明确授权，不修改 WidgetBlueprint、Lua、DataTable、.uasset、.umap 或其他 UGC 工程文件。"
    )
}

pub fn build_codex_new_task_url(workspace: &Path, prompt: &str) -> Result<String, String> {
    if !workspace.is_absolute() {
        return Err("Codex task workspace must be absolute".into());
    }
    let workspace = workspace
        .to_str()
        .ok_or_else(|| "Codex task workspace is not valid Unicode".to_string())?;
    let mut url = url::Url::parse("codex://threads/new")
        .map_err(|error| format!("could not initialize Codex new-task URL: {error}"))?;
    url.query_pairs_mut()
        .append_pair("prompt", prompt)
        .append_pair("path", workspace);
    Ok(url.into())
}

pub fn is_codex_desktop_executable_path(path: &str) -> bool {
    let normalized = path.replace('/', "\\").to_ascii_lowercase();
    normalized.ends_with(r"\app\chatgpt.exe") && normalized.contains(r"\windowsapps\openai.codex_")
}

pub fn submit_foreground_codex_prompt(timeout: Duration) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        windows::submit_foreground_codex_prompt(timeout)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = timeout;
        Ok(false)
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::is_codex_desktop_executable_path;
    use std::thread;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, KEYEVENTF_KEYUP, VK_RETURN,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    pub fn submit_foreground_codex_prompt(timeout: Duration) -> Result<bool, String> {
        let deadline = Instant::now() + timeout;
        let stable_duration = Duration::from_millis(1_500);
        let mut stable_since = None;
        while Instant::now() < deadline {
            if foreground_is_codex_desktop() {
                let since = stable_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= stable_duration {
                    if !foreground_is_codex_desktop() {
                        return Ok(false);
                    }
                    unsafe {
                        keybd_event(VK_RETURN as u8, 0, 0, 0);
                        keybd_event(VK_RETURN as u8, 0, KEYEVENTF_KEYUP, 0);
                    }
                    return Ok(true);
                }
            } else {
                stable_since = None;
            }
            thread::sleep(Duration::from_millis(100));
        }
        Ok(false)
    }

    fn foreground_is_codex_desktop() -> bool {
        foreground_executable_path()
            .as_deref()
            .is_some_and(is_codex_desktop_executable_path)
    }

    fn foreground_executable_path() -> Option<String> {
        unsafe {
            let window = GetForegroundWindow();
            if window.is_null() {
                return None;
            }
            let mut process_id = 0;
            GetWindowThreadProcessId(window, &mut process_id);
            if process_id == 0 {
                return None;
            }
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
            if process.is_null() {
                return None;
            }
            let mut buffer = vec![0_u16; 32_768];
            let mut length = buffer.len() as u32;
            let read = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length);
            CloseHandle(process);
            if read == 0 || length == 0 {
                return None;
            }
            Some(String::from_utf16_lossy(&buffer[..length as usize]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn builds_a_new_desktop_thread_url_with_the_delivery_prompt_and_workspace() {
        let prompt = build_delivery_prompt(
            PathBuf::from(r"C:\session\delivery-request.json").as_path(),
            PathBuf::from(r"C:\skill\report_ui_workflow_progress.py").as_path(),
            PathBuf::from(r"C:\session").as_path(),
            "currency-exchange",
        );
        assert!(prompt.contains(r"C:\session\delivery-request.json"));
        assert!(prompt.contains("oasis-wiki"));
        assert!(prompt.contains("新的 Codex 任务"));
        assert!(prompt.contains("冻结证据"));
        assert!(prompt.contains("精确 load_path"));
        assert!(prompt.contains("PRV"));
        assert!(prompt.contains("目标漂移"));
        assert!(prompt.contains("report_ui_workflow_progress.py"));
        assert!(prompt.contains("--session-dir"));
        assert!(prompt.contains(r"C:\session"));
        assert!(prompt.contains("--task-id"));
        assert!(prompt.contains("currency-exchange"));
        assert!(prompt.contains("--stage"));
        assert!(prompt.contains("--status"));
        assert!(prompt.contains("--message"));
        let new_task_url =
            build_codex_new_task_url(PathBuf::from(r"C:\RedCliff").as_path(), &prompt).unwrap();
        let parsed = url::Url::parse(&new_task_url).unwrap();
        assert_eq!(parsed.scheme(), "codex");
        assert_eq!(parsed.host_str(), Some("threads"));
        assert_eq!(parsed.path(), "/new");
        assert_eq!(
            parsed
                .query_pairs()
                .find(|(key, _)| key == "prompt")
                .map(|(_, value)| value.into_owned()),
            Some(prompt)
        );
        assert_eq!(
            parsed
                .query_pairs()
                .find(|(key, _)| key == "path")
                .map(|(_, value)| value.into_owned()),
            Some(r"C:\RedCliff".into())
        );
    }

    #[test]
    fn builds_distinct_new_ui_source_prompts() {
        let generate = build_ui_source_prompt(UiSourceMode::Generate);
        assert!(generate.contains("新的 Codex 任务"));
        assert!(generate.contains("oasis-wiki"));
        assert!(generate.contains("生成新的 UI"));
        assert!(generate.contains("SOURCE"));
        assert!(generate.contains("不要继续旧 UI"));

        let import = build_ui_source_prompt(UiSourceMode::Import);
        assert!(import.contains("新的 Codex 任务"));
        assert!(import.contains("oasis-wiki"));
        assert!(import.contains("已有 UI 图片"));
        assert!(import.contains("SOURCE"));
        assert!(import.contains("不要继续旧 UI"));
        assert_ne!(generate, import);
    }

    #[test]
    fn rejects_a_relative_workspace_for_a_new_desktop_thread() {
        assert!(build_codex_new_task_url(Path::new("RedCliff"), "prompt").is_err());
    }

    #[test]
    fn recognizes_only_the_packaged_codex_desktop_executable() {
        assert!(is_codex_desktop_executable_path(
            r"C:\Program Files\WindowsApps\OpenAI.Codex_26.810.7004.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe"
        ));
        assert!(!is_codex_desktop_executable_path(
            r"C:\Program Files\ChatGPT\ChatGPT.exe"
        ));
        assert!(!is_codex_desktop_executable_path(
            r"C:\Program Files\WindowsApps\OpenAI.Codex_26.810.7004.0_x64__2p2nqsd0c76g0\app\codex.exe"
        ));
    }

    #[test]
    fn resolves_the_reporter_from_the_bundled_resource_directory() {
        let resource_dir = PathBuf::from(r"C:\Program Files\Oasis Companion");

        assert_eq!(
            bundled_reporter_path(&resource_dir),
            resource_dir
                .join("resources")
                .join("skill")
                .join("scripts")
                .join("cowart-ui")
                .join("component-extractor")
                .join("report_ui_workflow_progress.py")
        );
    }
}
