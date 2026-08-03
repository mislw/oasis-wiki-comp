//! GitHub-backed update checks and one-click Skill updates.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::config;
use crate::skill::MultiTargetStatus;
use crate::skill::CURRENT_SKILL_VERSION;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateSource {
    Release,
    Commit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub checked: bool,
    pub update_available: bool,
    pub source: Option<UpdateSource>,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub latest_revision: Option<String>,
    pub installed_revision: Option<String>,
    pub latest_url: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInstallResult {
    pub status: UpdateStatus,
    pub skill_status: MultiTargetStatus,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    zipball_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubRepo {
    default_branch: String,
}

#[derive(Debug, Deserialize)]
struct GithubCommit {
    sha: String,
    html_url: String,
}

struct GithubUpdateCandidate {
    status: UpdateStatus,
    zipball_url: String,
}

pub async fn check(app: &AppHandle) -> UpdateStatus {
    let (repo, installed_revision) = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().unwrap();
        (
            settings.updates.github_repo.clone(),
            settings.updates.installed_revision.clone(),
        )
    };

    let result = check_github(&repo, installed_revision.as_deref()).await;
    let status = match result {
        Ok(status) => status,
        Err(error) => UpdateStatus {
            checked: false,
            update_available: false,
            source: None,
            current_version: CURRENT_SKILL_VERSION.to_string(),
            latest_version: None,
            latest_revision: None,
            installed_revision,
            latest_url: None,
            error: Some(error),
        },
    };

    {
        let st = app.state::<AppState>();
        st.update_available.store(
            status.update_available,
            std::sync::atomic::Ordering::Relaxed,
        );

        let mut settings = st.settings.lock().unwrap();
        settings.updates.last_check_at = Some(now_epoch_seconds());
        settings.updates.latest_version = status.latest_version.clone();
        settings.updates.latest_revision = status.latest_revision.clone();
        settings.updates.installed_revision = status.installed_revision.clone();
        settings.updates.latest_url = status.latest_url.clone();
        settings.updates.update_available = status.update_available;
        settings.updates.last_error = status.error.clone();
        if let Err(e) = config::save(&settings) {
            log::warn!("failed to save update status: {}", e);
        }
    }

    let new_state = app.state::<AppState>().compute_state();
    let changed = {
        let st = app.state::<AppState>();
        let mut bs = st.ball_state.lock().unwrap();
        let changed = *bs != new_state;
        *bs = new_state;
        changed
    };
    if changed {
        crate::ball::apply_state(app, new_state);
    }

    status
}

pub async fn install_latest(app: &AppHandle) -> Result<UpdateInstallResult, String> {
    let (repo, targets, installed_revision) = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().unwrap();
        (
            settings.updates.github_repo.clone(),
            settings.skill.targets.clone(),
            settings.updates.installed_revision.clone(),
        )
    };

    let candidate = check_github_candidate(&repo, installed_revision.as_deref()).await?;
    let skill_root = download_and_extract_skill(&candidate.zipball_url).await?;
    let skill_status = crate::skill::install_skill_from_dir(&skill_root, &targets)?;
    let installed_revision = candidate.status.latest_revision.clone();

    {
        let st = app.state::<AppState>();
        *st.skill_status.lock().unwrap() = skill_status.clone();

        st.update_available
            .store(false, std::sync::atomic::Ordering::Relaxed);

        let mut settings = st.settings.lock().unwrap();
        settings.updates.last_check_at = Some(now_epoch_seconds());
        settings.updates.latest_version = candidate.status.latest_version.clone();
        settings.updates.latest_revision = candidate.status.latest_revision.clone();
        settings.updates.installed_revision = installed_revision.clone();
        settings.updates.latest_url = candidate.status.latest_url.clone();
        settings.updates.update_available = false;
        settings.updates.last_error = None;
        settings.skill.installed_version = skill_status
            .aggregate()
            .installed_version()
            .map(String::from);
        if let Err(e) = config::save(&settings) {
            log::warn!("failed to save update install status: {}", e);
        }
    }

    let new_state = app.state::<AppState>().compute_state();
    let changed = {
        let st = app.state::<AppState>();
        let mut bs = st.ball_state.lock().unwrap();
        let changed = *bs != new_state;
        *bs = new_state;
        changed
    };
    if changed {
        crate::ball::apply_state(app, new_state);
    }

    let mut status = candidate.status;
    status.update_available = false;
    status.installed_revision = installed_revision;
    status.error = None;
    Ok(UpdateInstallResult {
        status,
        skill_status,
    })
}

async fn check_github(
    repo: &str,
    installed_revision: Option<&str>,
) -> Result<UpdateStatus, String> {
    Ok(check_github_candidate(repo, installed_revision)
        .await?
        .status)
}

async fn check_github_candidate(
    repo: &str,
    installed_revision: Option<&str>,
) -> Result<GithubUpdateCandidate, String> {
    let repo = normalize_repo(repo)?;
    let client = reqwest::Client::builder()
        .user_agent("Oasis-Companion/0.1")
        .build()
        .map_err(|e| e.to_string())?;

    let release_url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let release_response = client
        .get(&release_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if release_response.status().is_success() {
        let release: GithubRelease = release_response
            .json()
            .await
            .map_err(|e| format!("invalid release response: {e}"))?;
        let tag = release.tag_name.trim_start_matches('v').to_string();
        return Ok(GithubUpdateCandidate {
            zipball_url: release.zipball_url,
            status: UpdateStatus {
                checked: true,
                update_available: version_is_newer(&tag, CURRENT_SKILL_VERSION),
                source: Some(UpdateSource::Release),
                current_version: CURRENT_SKILL_VERSION.to_string(),
                latest_version: Some(tag),
                latest_revision: None,
                installed_revision: installed_revision.map(String::from),
                latest_url: Some(release.html_url),
                error: None,
            },
        });
    }

    if release_response.status().as_u16() != 404 {
        return Err(format!(
            "GitHub release check failed: HTTP {}",
            release_response.status()
        ));
    }

    let repo_url = format!("https://api.github.com/repos/{repo}");
    let repo_meta: GithubRepo = get_json(&client, &repo_url).await?;
    let commit_url = format!(
        "https://api.github.com/repos/{repo}/commits/{}",
        repo_meta.default_branch
    );
    let commit: GithubCommit = get_json(&client, &commit_url).await?;
    let short_sha = commit.sha.chars().take(12).collect::<String>();
    let update_available = revision_is_newer(&short_sha, installed_revision);
    let zipball_url = format!(
        "https://api.github.com/repos/{repo}/zipball/{}",
        repo_meta.default_branch
    );

    Ok(GithubUpdateCandidate {
        zipball_url,
        status: UpdateStatus {
            checked: true,
            update_available,
            source: Some(UpdateSource::Commit),
            current_version: CURRENT_SKILL_VERSION.to_string(),
            latest_version: None,
            latest_revision: Some(short_sha),
            installed_revision: installed_revision.map(String::from),
            latest_url: Some(commit.html_url),
            error: None,
        },
    })
}

async fn download_and_extract_skill(zipball_url: &str) -> Result<PathBuf, String> {
    let client = reqwest::Client::builder()
        .user_agent("Oasis-Companion/0.1")
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(zipball_url)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("download failed: HTTP {}", response.status()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("download body failed: {e}"))?;

    let extract_dir = std::env::temp_dir().join(format!(
        "oasis-wiki-update-{}-{}",
        std::process::id(),
        now_epoch_seconds()
    ));
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;

    extract_zip(&bytes, &extract_dir)?;
    find_skill_root(&extract_dir).ok_or_else(|| {
        format!(
            "downloaded archive does not contain a usable oasis-wiki Skill at {:?}",
            extract_dir
        )
    })
}

fn extract_zip(bytes: &[u8], dst: &Path) -> Result<(), String> {
    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("invalid zip: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(enclosed) = file.enclosed_name().map(|p| p.to_owned()) else {
            continue;
        };
        let out_path = dst.join(enclosed);
        if file.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = fs::File::create(&out_path).map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
        std::io::copy(&mut Cursor::new(buffer), &mut out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn find_skill_root(extract_dir: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    collect_skill_roots(extract_dir, &mut candidates);

    candidates
        .iter()
        .find(|path| path.file_name().and_then(|name| name.to_str()) == Some("oasis-wiki"))
        .cloned()
        .or_else(|| candidates.into_iter().next())
}

fn collect_skill_roots(dir: &Path, out: &mut Vec<PathBuf>) {
    if dir.join("SKILL.md").exists() {
        out.push(dir.to_path_buf());
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_skill_roots(&path, out);
            }
        }
    }
}

async fn get_json<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    url: &str,
) -> Result<T, String> {
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("GitHub request failed: HTTP {}", response.status()));
    }
    response
        .json::<T>()
        .await
        .map_err(|e| format!("invalid GitHub response: {e}"))
}

fn normalize_repo(repo: &str) -> Result<String, String> {
    let repo = repo
        .trim()
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/")
        .trim_end_matches('/')
        .trim_end_matches(".git");
    if repo.split('/').count() == 2 {
        Ok(repo.to_string())
    } else {
        Err("GitHub repo must look like owner/name".into())
    }
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let latest = version_parts(latest);
    let current = version_parts(current);
    latest > current
}

fn revision_is_newer(latest: &str, installed: Option<&str>) -> bool {
    let latest = latest.trim();
    if latest.is_empty() {
        return false;
    }
    match installed.map(str::trim).filter(|s| !s.is_empty()) {
        Some(installed) => latest != installed,
        None => true,
    }
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u32>().unwrap_or(0))
        .collect()
}

fn now_epoch_seconds() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_github_url_to_owner_repo() {
        assert_eq!(
            normalize_repo("https://github.com/mislw/oasis-wiki/").unwrap(),
            "mislw/oasis-wiki"
        );
    }

    #[test]
    fn compares_simple_versions() {
        assert!(version_is_newer("0.1.1", "0.1.0"));
        assert!(!version_is_newer("0.1.0", "0.1.0"));
        assert!(!version_is_newer("0.0.9", "0.1.0"));
    }

    #[test]
    fn compares_commit_revisions() {
        assert!(revision_is_newer("abc123", None));
        assert!(revision_is_newer("abc123", Some("def456")));
        assert!(!revision_is_newer("abc123", Some("abc123")));
        assert!(!revision_is_newer("", Some("abc123")));
    }
}
