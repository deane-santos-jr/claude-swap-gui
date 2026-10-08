use crate::cswap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;

const LABEL: &str = "com.cswap.auto";
const LOG_TAIL_BYTES: u64 = 64 * 1024;

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

fn plist_path() -> PathBuf {
    home()
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"))
}

pub fn log_path() -> PathBuf {
    home().join("Library/Logs").join(format!("{LABEL}.log"))
}

fn domain() -> String {
    let uid = String::from_utf8_lossy(
        &Command::new("id")
            .arg("-u")
            .output()
            .map(|o| o.stdout)
            .unwrap_or_default(),
    )
    .trim()
    .to_string();
    format!("gui/{uid}")
}

fn cswap_path() -> Result<String, String> {
    let output = Command::new("sh")
        .args(["-c", &format!("command -v {}", cswap::binary())])
        .output()
        .map_err(|e| e.to_string())?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return Err("cswap is not on PATH".into());
    }
    Ok(path)
}

fn plist(cswap: &str) -> String {
    let log = log_path().display().to_string();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array><string>{cswap}</string><string>auto</string><string>--json</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ProcessType</key><string>Background</string>
  <key>StandardOutPath</key><string>{log}</string>
  <key>StandardErrorPath</key><string>{log}</string>
</dict>
</plist>
"#
    )
}

fn launchctl(args: &[&str]) -> Result<String, String> {
    let output = Command::new("launchctl")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if output.status.success() {
        return Ok(stdout);
    }
    Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
}

pub fn is_running() -> bool {
    launchctl(&["print", &format!("{}/{LABEL}", domain())])
        .map(|out| out.contains("state = running"))
        .unwrap_or(false)
}

pub fn install() -> Result<(), String> {
    let cswap = cswap_path()?;
    let path = plist_path();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&path, plist(&cswap)).map_err(|e| e.to_string())?;
    let _ = launchctl(&["bootout", &format!("{}/{LABEL}", domain())]);
    launchctl(&["bootstrap", &domain(), &path.display().to_string()])?;
    launchctl(&["kickstart", "-k", &format!("{}/{LABEL}", domain())])?;
    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    let _ = launchctl(&["bootout", &format!("{}/{LABEL}", domain())]);
    let path = plist_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn last_log_line() -> Option<String> {
    let tail = read_log_tail(&log_path(), LOG_TAIL_BYTES).ok()?;
    last_meaningful_line(&tail)
}

fn read_log_tail(path: &Path, max_bytes: u64) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let start = file.metadata()?.len().saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    if start == 0 {
        return Ok(text);
    }
    Ok(without_first_line(&text).to_string())
}

fn without_first_line(text: &str) -> &str {
    text.split_once('\n').map_or("", |(_, rest)| rest)
}

fn last_meaningful_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .rev()
        .find(|l| !l.is_empty() && !is_malloc_stack_logging_noise(l))
        .map(str::to_string)
}

fn is_malloc_stack_logging_noise(line: &str) -> bool {
    line.contains("MallocStackLogging:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_log_line_skips_trailing_malloc_noise() {
        let text = "{\"event\":\"no-switch\"}\n\
            Python(60742) MallocStackLogging: can't turn off malloc stack logging because it was not enabled.\n\n";
        assert_eq!(
            last_meaningful_line(text).as_deref(),
            Some("{\"event\":\"no-switch\"}")
        );
    }

    #[test]
    fn last_log_line_keeps_other_stderr_text() {
        let text = "{\"event\":\"poll\"}\nTraceback (most recent call last):\n";
        assert_eq!(
            last_meaningful_line(text).as_deref(),
            Some("Traceback (most recent call last):")
        );
    }

    fn log_file_with(name: &str, contents: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("cswap-gui-{name}-{}.log", std::process::id()));
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn log_tail_drops_the_line_cut_by_the_byte_limit() {
        let path = log_file_with("cut", "first line\nsecond\nthird\n");
        assert_eq!(read_log_tail(&path, 10).unwrap(), "third\n");
    }

    #[test]
    fn log_tail_of_a_short_file_is_the_whole_file() {
        let path = log_file_with("short", "only\n");
        assert_eq!(read_log_tail(&path, 1024).unwrap(), "only\n");
    }

    #[test]
    fn login_item_runs_cswap_auto_with_json_output() {
        assert!(plist("/bin/cswap").contains(
            "<array><string>/bin/cswap</string><string>auto</string><string>--json</string></array>"
        ));
    }
}
