// Output parsing — extracts structured data from steamcmd VDF-like output.

use std::collections::HashMap;

use crate::{AppInfo, AppStatus, DownloadProgress};

/// Extract quoted `"key" "value"` pairs from steamcmd VDF-like output.
pub(crate) fn extract_kv_pairs(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        // Match lines like: "key"		"value"
        let parts: Vec<&str> = trimmed.split('"').collect();
        // A valid kv line splits into ["", key, sep, value, ""]
        if parts.len() >= 5 {
            let key = parts[1].to_string();
            let value = parts[3].to_string();
            // First occurrence wins (outermost section).
            map.entry(key).or_insert(value);
        }
    }
    map
}

/// Parse `app_info_print` output into an [`AppInfo`].
pub(crate) fn parse_app_info(app_id: &str, stdout: &str) -> AppInfo {
    let kv = extract_kv_pairs(stdout);
    AppInfo {
        app_id: app_id.into(),
        name: kv.get("name").cloned(),
        build_id: kv.get("buildid").cloned(),
        raw: kv,
    }
}

/// Parse `app_status` output into an [`AppStatus`].
pub(crate) fn parse_app_status(app_id: &str, stdout: &str) -> AppStatus {
    let kv = extract_kv_pairs(stdout);
    AppStatus {
        app_id: app_id.into(),
        name: kv.get("name").cloned(),
        install_dir: kv.get("installdir").cloned(),
        build_id: kv.get("buildid").cloned(),
        size_on_disk: kv.get("SizeOnDisk").and_then(|v| v.parse().ok()),
        state_flags: kv.get("StateFlags").and_then(|v| v.parse().ok()),
        update_success: kv.get("UpdateResult").map(|v| v == "0"),
    }
}

/// Try to parse a steamcmd progress line into a [`DownloadProgress`].
///
/// Expected format:
/// ` Update state (0x61) downloading, progress: 20.00 (193343132 / 966663354)`
pub(crate) fn parse_progress(line: &str) -> Option<DownloadProgress> {
    let trimmed = line.trim();

    // Must start with "Update state"
    let rest = trimmed.strip_prefix("Update state")?;

    // Extract state name: everything between ") " and ","
    let after_paren = rest.split(") ").nth(1)?;
    let (state, after_state) = after_paren.split_once(',')?;
    let state = state.trim().to_string();

    // Extract percent: "progress: XX.XX"
    let after_progress = after_state.strip_prefix(" progress: ")?;
    let (pct_str, rest) = after_progress.split_once(' ')?;
    let percent: f64 = pct_str.parse().ok()?;

    // Extract bytes: "(current / total)"
    let inner = rest.trim_start_matches('(').trim_end_matches(')').trim();
    let (current_str, total_str) = inner.split_once(" / ")?;
    let current_bytes: u64 = current_str.trim().parse().ok()?;
    let total_bytes: u64 = total_str.trim().parse().ok()?;

    Some(DownloadProgress {
        state,
        percent,
        current_bytes,
        total_bytes,
    })
}
