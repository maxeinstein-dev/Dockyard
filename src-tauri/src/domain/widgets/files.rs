//! The Arquivos (F9 of estudo 0002): an item of the dock whose panel shows
//! the documents opened last, in any program, as Windows keeps them in its
//! Recent folder; to open again or drag out.
//!
//! Declared in spec 0003 §2.5 as "the list of a folder", which a folder
//! pinned to the dock does since spec 0025: the kind kept its name and
//! became the recent files. It has nothing to keep, so the config is empty
//! and stays `{}`; a config of the old shape (with a `folder`) reads as it.

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// How many recent files the panel shows.
pub const MAX_SHOWN: usize = 16;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FilesConfig {}

impl FilesConfig {
    /// Always valid: there is nothing in it.
    pub fn validate(&self) -> Result<(), WidgetError> {
        Ok(())
    }
}

/// The recent files to show from the targets of the Recent folder's links,
/// the newest first: each file once, at most `MAX_SHOWN`. `keep` says
/// whether a target is still a file (the caller looks at the disk).
pub fn recent(targets: Vec<String>, keep: impl Fn(&str) -> bool) -> Vec<String> {
    let mut shown: Vec<String> = Vec::new();
    for target in targets {
        if shown.len() == MAX_SHOWN {
            break;
        }
        let known = shown.iter().any(|seen| seen.eq_ignore_ascii_case(&target));
        if !target.is_empty() && !known && keep(&target) {
            shown.push(target);
        }
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_is_empty_and_reads_the_old_shape() {
        assert_eq!(serde_json::to_string(&FilesConfig::default()).unwrap(), "{}");
        let old: FilesConfig = serde_json::from_str(r#"{"folder": "C:\\"}"#).unwrap();
        assert_eq!(old, FilesConfig::default());
        assert_eq!(old.validate(), Ok(()));
    }

    #[test]
    fn each_file_once_the_newest_first_only_those_kept() {
        let targets = vec![
            r"C:\a.txt".to_owned(),
            String::new(),
            r"C:\gone.txt".to_owned(),
            r"c:\A.TXT".to_owned(),
            r"C:\b.pdf".to_owned(),
        ];
        assert_eq!(
            recent(targets, |target| !target.contains("gone")),
            vec![r"C:\a.txt".to_owned(), r"C:\b.pdf".to_owned()]
        );
    }

    #[test]
    fn no_more_than_shown() {
        let targets = (0..40).map(|n| format!(r"C:\{n}.txt")).collect();
        assert_eq!(recent(targets, |_| true).len(), MAX_SHOWN);
    }
}
