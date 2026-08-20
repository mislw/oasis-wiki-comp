//! Skill install-status detection + (re)install from the bundled resource.
//!
//! The Skill bundle is shipped as a Tauri resource (`resources/skill/**`) and
//! installed to each enabled Agent target's skill directory (e.g.
//! `~/.codex/skills/oasis-wiki`, `~/.claude/commands/oasis-wiki`,
//! `~/.workbuddy/skills/oasis-wiki`). `SKILL.md` is the install marker;
//! `VERSION` is optional and used only for version-mismatch detection.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// The version shipped in the bundled Skill resource. Bump when the bundle
/// changes. If an installed Skill includes `VERSION`, status detection compares
/// it against this value.
pub const CURRENT_SKILL_VERSION: &str = "1.260820.2";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkillStatus {
    NotInstalled,
    Installed,
    /// Installed but `VERSION` != bundled version.
    VersionMismatch {
        installed: String,
        expected: String,
    },
}

impl SkillStatus {
    pub fn installed_version(&self) -> Option<&str> {
        match self {
            SkillStatus::Installed => Some(CURRENT_SKILL_VERSION),
            SkillStatus::VersionMismatch { installed, .. } => Some(installed),
            SkillStatus::NotInstalled => None,
        }
    }
}

/// Status of a single Agent target.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetStatus {
    pub target_id: String,
    pub display_name: String,
    pub status: SkillStatus,
}

/// Multi-target Skill installation status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct MultiTargetStatus {
    pub targets: Vec<TargetStatus>,
}

impl MultiTargetStatus {
    /// True if all enabled targets have the Skill installed.
    pub fn all_installed(&self) -> bool {
        !self.targets.is_empty()
            && self
                .targets
                .iter()
                .all(|t| t.status == SkillStatus::Installed)
    }

    /// True if any target is NotInstalled or VersionMismatch.
    #[allow(dead_code)]
    pub fn is_unhealthy(&self) -> bool {
        !self.all_installed()
    }

    /// Aggregate to a single SkillStatus for backward-compat consumers.
    pub fn aggregate(&self) -> SkillStatus {
        if self.targets.is_empty() {
            return SkillStatus::NotInstalled;
        }
        if self.all_installed() {
            return SkillStatus::Installed;
        }
        for t in &self.targets {
            if let SkillStatus::VersionMismatch {
                ref installed,
                ref expected,
            } = t.status
            {
                return SkillStatus::VersionMismatch {
                    installed: installed.clone(),
                    expected: expected.clone(),
                };
            }
        }
        SkillStatus::NotInstalled
    }
}

/// Detect Skill status across all enabled targets.
pub fn detect_status(targets: &[String]) -> MultiTargetStatus {
    let registry = crate::agent_registry::all_targets();
    let target_statuses: Vec<TargetStatus> = targets
        .iter()
        .map(|id| {
            let target = registry.iter().find(|t| &t.id == id);
            let (display_name, skill_dir) = match target {
                Some(t) => (t.display_name.clone(), t.skill_dir.clone()),
                None => (id.clone(), id.clone()),
            };
            let status = detect_single(&skill_dir);
            TargetStatus {
                target_id: id.clone(),
                display_name,
                status,
            }
        })
        .collect();
    MultiTargetStatus {
        targets: target_statuses,
    }
}

/// Detect whether a target has a usable Skill. `SKILL.md` is enough to avoid
/// false error states for GitHub-installed Skills that do not ship `VERSION`.
fn detect_single(skill_dir: &str) -> SkillStatus {
    let target = crate::config::expand_user_profile(skill_dir);
    detect_path(&target)
}

fn detect_path(target: &Path) -> SkillStatus {
    if !target.join("SKILL.md").exists() {
        return SkillStatus::NotInstalled;
    }

    let version_file = target.join("VERSION");
    if !version_file.exists() {
        return SkillStatus::Installed;
    }
    let installed = match fs::read_to_string(&version_file) {
        Ok(s) => s.trim().to_string(),
        Err(_) => return SkillStatus::Installed,
    };
    if installed.is_empty() {
        return SkillStatus::Installed;
    }
    if installed == CURRENT_SKILL_VERSION {
        SkillStatus::Installed
    } else {
        SkillStatus::VersionMismatch {
            installed,
            expected: CURRENT_SKILL_VERSION.to_string(),
        }
    }
}

/// (Re)install the Skill from the bundled resource into all enabled targets.
///
/// For each target, the resource bundle is staged and validated before an
/// atomic replacement. A failed activation restores the previous target.
/// Successful installation removes obsolete copies of the same Skill and
/// installer-owned backup or temporary directories.
///
/// Returns the post-install status for all targets.
fn resolve_skill_source(candidates: &[PathBuf]) -> Result<PathBuf, String> {
    for candidate in candidates {
        if candidate.is_dir() && candidate.join("SKILL.md").is_file() {
            return Ok(candidate.clone());
        }
    }

    let attempted = candidates
        .iter()
        .map(|candidate| format!("- {}", candidate.display()))
        .collect::<Vec<_>>()
        .join("\n");
    Err(format!(
        "bundled Skill resource not found; attempted trusted paths:\n{}",
        attempted
    ))
}

pub fn install_skill(app: &AppHandle, targets: &[String]) -> Result<MultiTargetStatus, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("resource_dir: {}", e))?;
    let candidates = vec![
        resource_dir.join("skill"),
        resource_dir.join("skills").join("oasis-wiki"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("skill"),
    ];
    let resource_root = resolve_skill_source(&candidates)?;
    log::info!("skill bundle resource: {:?}", resource_root);
    install_skill_from_dir(&resource_root, targets)
}

/// Install the Skill from a caller-provided directory into all enabled targets.
pub fn install_skill_from_dir(
    source_root: &Path,
    targets: &[String],
) -> Result<MultiTargetStatus, String> {
    if !source_root.exists() {
        return Err(format!("Skill source not found at {:?}", source_root));
    }
    if !source_root.join("SKILL.md").exists() {
        return Err(format!(
            "Skill source at {:?} does not contain SKILL.md",
            source_root
        ));
    }

    log::info!("installing skill bundle from {:?}", source_root);
    let registry = crate::agent_registry::all_targets();
    let mut results = Vec::new();

    for id in targets {
        let target = registry.iter().find(|t| &t.id == id);
        let (display_name, skill_dir, skill_roots) = match target {
            Some(t) => (
                t.display_name.clone(),
                t.skill_dir.clone(),
                t.skill_roots.clone(),
            ),
            None => (id.clone(), id.clone(), Vec::new()),
        };

        log::info!("installing skill to target: {} ({})", id, skill_dir);
        let status = install_single(source_root, &skill_dir, &skill_roots)?;
        results.push(TargetStatus {
            target_id: id.clone(),
            display_name,
            status,
        });
    }

    Ok(MultiTargetStatus { targets: results })
}

/// Install to a single target directory.
fn install_single(
    resource_root: &Path,
    skill_dir: &str,
    skill_roots: &[String],
) -> Result<SkillStatus, String> {
    let target = crate::config::expand_user_profile(skill_dir);
    let roots = skill_roots
        .iter()
        .map(|root| crate::config::expand_user_profile(root))
        .collect::<Vec<_>>();
    install_single_with_roots(resource_root, &target, &roots)
}

fn install_single_with_roots(
    resource_root: &Path,
    target: &Path,
    skill_roots: &[PathBuf],
) -> Result<SkillStatus, String> {
    let parent = target
        .parent()
        .ok_or_else(|| format!("invalid Skill target: {:?}", target))?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let tmp = parent.join(format!(".oasis-wiki.tmp.{}.{}", std::process::id(), nonce));
    let rollback = parent.join(format!(
        ".oasis-wiki.rollback.{}.{}",
        std::process::id(),
        nonce
    ));
    if tmp.exists() {
        fs::remove_dir_all(&tmp)
            .map_err(|error| format!("remove stale staged Skill {:?}: {error}", tmp))?;
    }
    if let Err(error) = copy_dir(resource_root, &tmp) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(format!("stage Skill at {:?}: {error}", tmp));
    }
    if !tmp.join("SKILL.md").exists() {
        let _ = fs::remove_dir_all(&tmp);
        return Err(format!(
            "staged Skill at {:?} does not contain SKILL.md",
            tmp
        ));
    }

    let had_previous = activate_staged_skill(&tmp, target, &rollback)?;
    let mut roots = skill_roots.to_vec();
    if !roots.iter().any(|root| root == parent) {
        roots.push(parent.to_path_buf());
    }
    if let Err(error) = cleanup_obsolete_skill_copies(target, &roots, &rollback) {
        let restore_error = restore_previous_skill(target, &rollback, had_previous).err();
        return Err(match restore_error {
            Some(restore_error) => format!("{error}; rollback failed: {restore_error}"),
            None => error,
        });
    }

    if had_previous {
        if let Err(error) = fs::remove_dir_all(&rollback) {
            let restore_error = restore_previous_skill(target, &rollback, true).err();
            return Err(match restore_error {
                Some(restore_error) => format!(
                    "remove rollback Skill {:?}: {error}; rollback failed: {restore_error}",
                    rollback
                ),
                None => format!("remove rollback Skill {:?}: {error}", rollback),
            });
        }
    }

    Ok(detect_path(target))
}

fn activate_staged_skill(staged: &Path, target: &Path, rollback: &Path) -> Result<bool, String> {
    let had_previous = target.exists();
    if had_previous {
        fs::rename(target, rollback)
            .map_err(|error| format!("back up current Skill {:?}: {error}", target))?;
    }

    if let Err(error) = fs::rename(staged, target) {
        let restore_error = if had_previous {
            fs::rename(rollback, target).err()
        } else {
            None
        };
        let _ = fs::remove_dir_all(staged);
        return Err(match restore_error {
            Some(restore_error) => format!(
                "activate staged Skill {:?}: {error}; restore failed: {restore_error}",
                staged
            ),
            None => format!("activate staged Skill {:?}: {error}", staged),
        });
    }

    Ok(had_previous)
}

fn restore_previous_skill(
    target: &Path,
    rollback: &Path,
    had_previous: bool,
) -> Result<(), String> {
    if target.exists() {
        fs::remove_dir_all(target)
            .map_err(|error| format!("remove failed Skill {:?}: {error}", target))?;
    }
    if had_previous && rollback.exists() {
        fs::rename(rollback, target)
            .map_err(|error| format!("restore previous Skill {:?}: {error}", rollback))?;
    }
    Ok(())
}

fn cleanup_obsolete_skill_copies(
    target: &Path,
    skill_roots: &[PathBuf],
    rollback: &Path,
) -> Result<(), String> {
    for root in skill_roots {
        if !root.exists() {
            continue;
        }
        let entries = fs::read_dir(root)
            .map_err(|error| format!("scan Skill directory {:?}: {error}", root))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("read Skill entry: {error}"))?;
            let file_type = entry
                .file_type()
                .map_err(|error| format!("inspect Skill entry {:?}: {error}", entry.path()))?;
            if !file_type.is_dir() || file_type.is_symlink() {
                continue;
            }

            let path = entry.path();
            if path == target || path == rollback {
                continue;
            }

            let name = entry.file_name();
            let name = name.to_string_lossy();
            let installer_residue = name.starts_with("oasis-wiki.bak.")
                || name.starts_with(".oasis-wiki.tmp.")
                || name.starts_with(".oasis-wiki.rollback.");
            let duplicate_skill = read_skill_name(&path).as_deref() == Some("oasis-wiki");
            if installer_residue || duplicate_skill {
                fs::remove_dir_all(&path)
                    .map_err(|error| format!("remove obsolete Skill {:?}: {error}", path))?;
                log::info!("removed obsolete Skill copy: {:?}", path);
            }
        }
    }
    Ok(())
}

fn read_skill_name(skill_dir: &Path) -> Option<String> {
    let contents = fs::read_to_string(skill_dir.join("SKILL.md")).ok()?;
    let mut lines = contents.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        let line = line.trim();
        if line == "---" {
            break;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim() == "name" {
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .or_else(|| {
                    value
                        .strip_prefix('\'')
                        .and_then(|value| value.strip_suffix('\''))
                })
                .unwrap_or(value);
            return Some(value.to_string());
        }
    }
    None
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "oasis-companion-skill-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn write_skill(dir: &Path, name: &str, version: &str, marker: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\n---\n\n# {marker}\n"),
        )
        .unwrap();
        fs::write(dir.join("VERSION"), version).unwrap();
    }

    #[test]
    fn resolve_skill_source_prefers_the_first_valid_candidate() {
        let root = test_dir("source-priority");
        let invalid = root.join("invalid");
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&invalid).unwrap();
        write_skill(&first, "oasis-wiki", CURRENT_SKILL_VERSION, "first");
        write_skill(&second, "oasis-wiki", CURRENT_SKILL_VERSION, "second");

        let selected = resolve_skill_source(&[invalid, first.clone(), second]).unwrap();

        assert_eq!(selected, first);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_skill_source_rejects_a_directory_without_skill_md() {
        let root = test_dir("source-marker");
        let candidate = root.join("candidate");
        fs::create_dir_all(&candidate).unwrap();

        let error = resolve_skill_source(std::slice::from_ref(&candidate)).unwrap_err();

        assert!(error.contains(candidate.to_string_lossy().as_ref()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_skill_source_error_lists_every_attempted_path() {
        let root = test_dir("source-error");
        let first = root.join("first");
        let second = root.join("second");

        let error = resolve_skill_source(&[first.clone(), second.clone()]).unwrap_err();

        assert!(error.contains("attempted trusted paths"));
        assert!(error.contains(first.to_string_lossy().as_ref()));
        assert!(error.contains(second.to_string_lossy().as_ref()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn detect_status_empty_targets() {
        let status = detect_status(&[]);
        assert!(status.targets.is_empty());
        assert!(status.is_unhealthy());
    }

    #[test]
    fn skill_md_without_version_is_installed() {
        let dir = std::env::temp_dir().join(format!(
            "oasis-companion-skill-detect-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), "---\nname: oasis-wiki\n---\n").unwrap();

        assert_eq!(detect_single(dir.to_str().unwrap()), SkillStatus::Installed);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn multi_target_all_installed() {
        let status = MultiTargetStatus {
            targets: vec![
                TargetStatus {
                    target_id: "codex".into(),
                    display_name: "Codex".into(),
                    status: SkillStatus::Installed,
                },
                TargetStatus {
                    target_id: "workbuddy".into(),
                    display_name: "WorkBuddy".into(),
                    status: SkillStatus::Installed,
                },
            ],
        };
        assert!(status.all_installed());
        assert!(!status.is_unhealthy());
        assert_eq!(status.aggregate(), SkillStatus::Installed);
    }

    #[test]
    fn multi_target_partial() {
        let status = MultiTargetStatus {
            targets: vec![
                TargetStatus {
                    target_id: "codex".into(),
                    display_name: "Codex".into(),
                    status: SkillStatus::Installed,
                },
                TargetStatus {
                    target_id: "workbuddy".into(),
                    display_name: "WorkBuddy".into(),
                    status: SkillStatus::NotInstalled,
                },
            ],
        };
        assert!(!status.all_installed());
        assert!(status.is_unhealthy());
        assert_eq!(status.aggregate(), SkillStatus::NotInstalled);
    }

    #[test]
    fn successful_install_removes_old_skill_copies_and_residue() {
        let root = test_dir("cleanup");
        let source = root.join("source");
        let commands_root = root.join("agent").join("commands");
        let skills_root = root.join("agent").join("skills");
        let target = commands_root.join("oasis-wiki");
        let legacy_named = skills_root.join("oasis-wiki");
        let legacy_aliased = skills_root.join("standalone-oasis-wiki");
        let unrelated = skills_root.join("oasis-companion-popup-test");
        let old_backup = commands_root.join("oasis-wiki.bak.100");
        let stale_tmp = skills_root.join(".oasis-wiki.tmp.200");

        write_skill(&source, "oasis-wiki", CURRENT_SKILL_VERSION, "new");
        write_skill(&target, "oasis-wiki", "0.1.0", "old");
        write_skill(&legacy_named, "oasis-wiki", "0.1.0", "legacy");
        write_skill(&legacy_aliased, "oasis-wiki", "0.1.0", "alias");
        write_skill(
            &unrelated,
            "oasis-companion-popup-test",
            "0.1.0",
            "unrelated",
        );
        write_skill(&old_backup, "oasis-wiki", "0.1.0", "backup");
        fs::create_dir_all(&stale_tmp).unwrap();

        let status = install_single_with_roots(
            &source,
            &target,
            &[commands_root.clone(), skills_root.clone()],
        )
        .unwrap();

        assert_eq!(status, SkillStatus::Installed);
        assert!(fs::read_to_string(target.join("SKILL.md"))
            .unwrap()
            .contains("# new"));
        assert!(!legacy_named.exists());
        assert!(!legacy_aliased.exists());
        assert!(!old_backup.exists());
        assert!(!stale_tmp.exists());
        assert!(unrelated.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn activation_failure_restores_existing_skill() {
        let root = test_dir("rollback");
        let target = root.join("oasis-wiki");
        let missing_staged = root.join("missing-staged");
        let rollback = root.join(".oasis-wiki.rollback.test");
        write_skill(&target, "oasis-wiki", "0.1.0", "old");

        let error = activate_staged_skill(&missing_staged, &target, &rollback).unwrap_err();

        assert!(error.contains("activate staged Skill"));
        assert!(target.exists());
        assert!(fs::read_to_string(target.join("SKILL.md"))
            .unwrap()
            .contains("# old"));
        assert!(!rollback.exists());

        let _ = fs::remove_dir_all(root);
    }
}
