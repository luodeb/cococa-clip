use std::fs;
use std::path::PathBuf;

const LAUNCH_AGENT_LABEL: &str = "com.cococa.clip";

pub fn is_enabled() -> Result<bool, String> {
    let path = launch_agent_path()?;
    Ok(path.exists())
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let path = launch_agent_path()?;

    if enabled {
        let executable = std::env::current_exe()
            .map_err(|error| format!("读取当前可执行路径失败: {error}"))?;

        let parent = path
            .parent()
            .ok_or_else(|| "无效的 LaunchAgent 路径".to_owned())?;

        fs::create_dir_all(parent).map_err(|error| format!("创建 LaunchAgents 目录失败: {error}"))?;

        let plist = render_launch_agent_plist(&executable);
        fs::write(&path, plist).map_err(|error| format!("写入开机自启配置失败: {error}"))?;
    } else if path.exists() {
        fs::remove_file(&path).map_err(|error| format!("移除开机自启配置失败: {error}"))?;
    }

    Ok(())
}

fn launch_agent_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|error| format!("读取 HOME 失败: {error}"))?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LAUNCH_AGENT_LABEL}.plist")))
}

fn render_launch_agent_plist(executable: &PathBuf) -> String {
    let executable = xml_escape(&executable.display().to_string());
    let label = xml_escape(LAUNCH_AGENT_LABEL);

    format!(
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{executable}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
