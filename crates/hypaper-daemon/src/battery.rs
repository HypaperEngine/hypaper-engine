//! Battery status detection via Linux sysfs.

/// Returns `true` if the system is currently running on battery power.
///
/// Scans `/sys/class/power_supply/AC*/online`: a value of `1` means the AC
/// adapter is connected (not on battery); `0` means the adapter is
/// disconnected (on battery).
///
/// Returns `false` when no AC adapter entry is found, which covers desktop
/// systems that have no battery at all.
pub fn is_on_battery() -> bool {
    let power_supply = std::path::Path::new("/sys/class/power_supply");

    let entries = match std::fs::read_dir(power_supply) {
        Ok(e) => e,
        Err(_) => {
            tracing::debug!("could not read /sys/class/power_supply, assuming AC power");
            return false;
        }
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if !name_str.starts_with("AC") {
            continue;
        }

        let online_path = entry.path().join("online");
        match std::fs::read_to_string(&online_path) {
            Ok(content) => {
                let online = content.trim() == "1";
                tracing::debug!(
                    adapter = %name_str,
                    online,
                    "AC adapter status read from sysfs",
                );
                return !online;
            }
            Err(_) => continue,
        }
    }

    tracing::debug!("no AC adapter found in sysfs, assuming AC power (desktop)");
    false
}
