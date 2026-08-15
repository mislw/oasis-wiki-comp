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
pub const CURRENT_SKILL_VERSION: &str = "1.260815.3";

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
/// For each target:
/// - Backs up an existing target to `oasis-wiki.bak.<ts>` (keeps last 3).
/// - Copies the resource bundle via a temp dir + atomic rename.
///
/// Returns the post-install status for all targets.
pub fn install_skill(app: &AppHandle, targets: &[String]) -> Result<MultiTargetStatus, String> {
    let resource_root = app
        .path()
        .resource_dir()
        .map_err(|e| format!("resource_dir: {}", e))?
        .join("skill");
    log::info!("skill bundle resource: {:?}", resource_root);

    if !resource_root.exists() {
        return Err(format!(
            "bundled Skill resource not found at {:?}. Build with `resources/skill/**`.",
            resource_root
        ));
    }
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
        let (display_name, skill_dir) = match target {
            Some(t) => (t.display_name.clone(), t.skill_dir.clone()),
            None => (id.clone(), id.clone()),
        };

        log::info!("installing skill to target: {} ({})", id, skill_dir);
        let status = install_single(source_root, &skill_dir)?;
        results.push(TargetStatus {
            target_id: id.clone(),
            display_name,
            status,
        });
    }

    Ok(MultiTargetStatus { targets: results })
}

/// Install to a single target directory.
fn install_single(resource_root: &Path, skill_dir: &str) -> Result<SkillStatus, String> {
    let target = crate::config::expand_user_profile(skill_dir);
    let parent = target
        .parent()
        .ok_or_else(|| format!("invalid skill_dir: {}", skill_dir))?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    // Back up existing target.
    if target.exists() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let bak = target.with_file_name(format!("oasis-wiki.bak.{}", ts));
        let _ = fs::rename(&target, &bak);
        log::info!("backed up existing skill to {:?}", bak);
        prune_backups(parent);
    }

    // Copy into a temp sibling, then rename atomically.
    let tmp = parent.join(format!(".oasis-wiki.tmp.{}", std::process::id()));
    if tmp.exists() {
        let _ = fs::remove_dir_all(&tmp);
    }
    copy_dir(resource_root, &tmp).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &target).map_err(|e| e.to_string())?;

    Ok(detect_single(skill_dir))
}

fn prune_backups(parent: &Path) {
    let mut backups: Vec<(PathBuf, u64)> = Vec::new();
    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(ts) = name.strip_prefix("oasis-wiki.bak.") {
                if let Ok(ts) = ts.parse::<u64>() {
                    backups.push((entry.path(), ts));
                }
            }
        }
    }
    backups.sort_by_key(|b| std::cmp::Reverse(b.1));
    for (path, _) in backups.into_iter().skip(3) {
        let _ = fs::remove_dir_all(&path);
    }
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
}
