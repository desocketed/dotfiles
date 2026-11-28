//! Routing streams to GAME or CHAT: rules from the daemon's config, handed to a small
//! WirePlumber script that sets each matching stream's target as it appears, before WirePlumber
//! links it, so a stream never plays on the wrong sink first.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use anyhow::anyhow;
use novamix::Result;
use novamix::mix::Sink;

use crate::names::Names;

/// The files the daemon owns; edits to them are overwritten on the next start.
const FRAGMENT: &str = "wireplumber/wireplumber.conf.d/90-novamix.conf";
const SCRIPT: &str = "wireplumber/scripts/novamix.lua";
const SCRIPT_TEXT: &str = include_str!("novamix.lua");

/// The rules apply to playback streams only: a microphone stream with `media.role =
/// Communication` must not capture CHAT's monitor.
const PLAYBACK: &str = "Stream/Output/Audio";

/// Used when the config has no `[[route]]`. A value starting with `~` is a POSIX extended
/// regular expression, as in WirePlumber's own rules.
const DEFAULTS: &str = r#"
# Voice chat
[[route]]
sink = "chat"
match = [
	{ "media.role" = "Communication" },
	{ "application.process.binary" = "~^([Dd]iscord(Canary|PTB)?|[Vv]esktop|[Ww]eb[Cc]ord|mumble|ts3client_linux_(amd64|x86)|TeamSpeak|yakvc)$" },
	{ "application.name" = "~^([Dd]iscord|[Vv]esktop|[Ww]eb[Cc]ord|Mumble|TeamSpeak.*|Yak ?VC.*)$" },
]

# Games: SDL tags its streams Game, Wine and Proton name them after the .exe, and gamescope
[[route]]
sink = "game"
match = [
	{ "media.role" = "Game" },
	{ "application.process.binary" = "~\\.(exe|EXE)$" },
	{ "application.name" = "~\\.(exe|EXE)$" },
	{ "application.process.binary" = "~^(wine|wine64|wine-preloader|wine64-preloader|gamescope)$" },
]

# Music players: GLOBAL, so the dial leaves them alone
[[route]]
sink = "global"
match = [
	{ "media.role" = "Music" },
	{ "application.process.binary" = "~^(mpd|[Ss]potify|[Ff]eishin|[Ss]upersonic)$" },
	{ "application.name" = "~^(Music Player Daemon|[Ss]potify|[Ff]eishin|[Ss]upersonic)$" },
]
"#;

/// Streams with all of one `matches` entry's properties go to `sink`.
#[derive(Debug, Eq, PartialEq)]
pub struct Rule {
    pub sink: Sink,
    pub matches: Vec<Vec<(String, String)>>,
}

impl From<&Rule> for novamix::ipc::Rule {
    fn from(rule: &Rule) -> Self {
        Self {
            sink: rule.sink,
            matches: rule.matches.clone(),
        }
    }
}

/// The text of `$XDG_CONFIG_HOME/novamix/config.toml`, empty when there is none.
pub fn read(path: Option<&Path>) -> Result<String> {
    let path = match path {
        Some(path) => path.to_path_buf(),
        None => dirs::config_dir()
            .ok_or(anyhow!("No XDG_CONFIG_HOME"))?
            .join("novamix/config.toml"),
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(anyhow!("Cannot read {}: {err}", path.display())),
    };
    Ok(text)
}

/// The config's rules, or the defaults when it has none.
pub fn load(text: &str) -> Result<Vec<Rule>> {
    let rules = parse(text)?;
    if rules.is_empty() {
        return parse(DEFAULTS);
    }
    Ok(rules)
}

/// `write_dial = false` stops a mix set from the socket being written to the base station.
pub fn write_dial(text: &str) -> Result<bool> {
    let table: toml::Table = text.parse().or(Err(anyhow!("Config is not valid TOML")))?;
    match table.get("write_dial") {
        None => Ok(true),
        Some(value) => value
            .as_bool()
            .ok_or(anyhow!("write_dial must be true or false")),
    }
}

/// `take_default = true` makes GLOBAL the default sink once the sinks are bound; off by default.
pub fn take_default(text: &str) -> Result<bool> {
    let table: toml::Table = text.parse().or(Err(anyhow!("Config is not valid TOML")))?;
    match table.get("take_default") {
        None => Ok(false),
        Some(value) => value
            .as_bool()
            .ok_or(anyhow!("take_default must be true or false")),
    }
}

/// `[[route]]` tables: `sink = "game" | "chat" | "global"`, `match = [{ prop = "value", .. }, ..]`.
pub fn parse(text: &str) -> Result<Vec<Rule>> {
    let table: toml::Table = text.parse().or(Err(anyhow!("Config is not valid TOML")))?;
    let Some(routes) = table.get("route") else {
        return Ok(Vec::new());
    };
    let routes = routes
        .as_array()
        .ok_or(anyhow!("route must be [[route]] tables"))?;
    let mut rules = Vec::new();
    for route in routes {
        let sink = match route.get("sink").and_then(|sink| sink.as_str()) {
            Some("game") => Sink::Game,
            Some("chat") => Sink::Chat,
            Some("global") => Sink::Global,
            sink => {
                return Err(anyhow!(
                    "route sink must be \"game\", \"chat\" or \"global\", not {sink:?}"
                ));
            }
        };
        let entries = route
            .get("match")
            .and_then(|entries| entries.as_array())
            .ok_or(anyhow!("route needs match = [{{ property = \"value\" }}]"))?;
        let mut matches = Vec::new();
        for entry in entries {
            let entry = entry.as_table().ok_or(anyhow!(
                "match entries are {{ property = \"value\" }} tables"
            ))?;
            let mut props = Vec::new();
            for (key, value) in entry {
                let value = value
                    .as_str()
                    .ok_or(anyhow!("match value for {key} must be a string"))?;
                props.push((key.clone(), value.to_string()));
            }
            matches.push(props);
        }
        rules.push(Rule { sink, matches });
    }
    Ok(rules)
}

/// The sink of the first rule that matches a stream's properties, as novamix.lua decides it.
/// A `~` value is a regular expression (`regex` syntax, close to POSIX ERE); one that does not
/// compile matches nothing.
pub fn matching(
    rules: &[novamix::ipc::Rule],
    props: &HashMap<String, String>,
) -> Option<Sink> {
    let matches = |(key, want): &(String, String)| {
        let Some(have) = props.get(key) else {
            return false;
        };
        match want.strip_prefix('~') {
            Some(pattern) => regex::Regex::new(pattern).is_ok_and(|re| re.is_match(have)),
            None => have == want,
        }
    };
    rules
        .iter()
        .find(|rule| rule.matches.iter().any(|entry| entry.iter().all(matches)))
        .map(|rule| rule.sink)
}

/// The rules as a WirePlumber 0.5 config fragment (SPA-JSON): the `novamix.rules` that
/// novamix.lua reads, and the component that loads it.
pub fn render(
    rules: &[Rule],
    names: &Names,
) -> String {
    let mut out =
        String::from("# Written by novamix-daemon from its config; edits are overwritten.\n");
    out.push_str("novamix.rules = [\n");
    for rule in rules {
        out.push_str("  {\n    matches = [\n");
        for props in &rule.matches {
            out.push_str(&format!("      {{ media.class = {}", quote(PLAYBACK)));
            for (key, value) in props {
                out.push_str(&format!(", {} = {}", quote(key), quote(value)));
            }
            out.push_str(" }\n");
        }
        out.push_str("    ]\n");
        out.push_str(&format!(
            "    actions = {{ novamix-route = {} }}\n",
            quote(&names.node_name(rule.sink))
        ));
        out.push_str("  }\n");
    }
    out.push_str("]\n");
    out.push_str(COMPONENT);
    out
}

const COMPONENT: &str = "\
wireplumber.components = [
  { name = novamix.lua, type = script/lua, provides = custom.novamix }
]
wireplumber.profiles = {
  main = { custom.novamix = required }
}
";

fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Writes the script and its config fragment, and restarts WirePlumber when either changed;
/// WirePlumber links the streams already playing again as it starts.
pub fn install(
    rules: &[Rule],
    names: &Names,
) -> Result<()> {
    let config = dirs::config_dir().ok_or(anyhow!("No XDG_CONFIG_HOME"))?;
    let data = dirs::data_dir().ok_or(anyhow!("No XDG_DATA_HOME"))?;
    let script = write(&data.join(SCRIPT), SCRIPT_TEXT)?;
    let fragment = write(&config.join(FRAGMENT), &render(rules, names))?;
    if !script && !fragment {
        log::debug!("WirePlumber routing is up to date");
        return Ok(());
    }
    log::info!("routing rules changed, restarting WirePlumber");
    let status = Command::new("systemctl")
        .args(["--user", "try-restart", "wireplumber.service"])
        .status();
    match status {
        Ok(status) if status.success() => {}
        Ok(status) => log::warn!("systemctl restart wireplumber: {status}; restart it yourself"),
        Err(err) => log::warn!("systemctl: {err}; restart WirePlumber yourself"),
    }
    Ok(())
}

/// Writes `text` unless the file already holds it; true when it wrote.
fn write(
    path: &Path,
    text: &str,
) -> Result<bool> {
    if std::fs::read_to_string(path).ok().as_deref() == Some(text) {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text).or(Err(anyhow!("Cannot write {}", path.display())))?;
    log::info!("wrote {}", path.display());
    Ok(true)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_defaults_parse() {
        let rules = parse(DEFAULTS).unwrap();
        assert!(rules.len() == 3);
        assert!(rules[0].sink == Sink::Chat);
        assert!(rules[0].matches[0] == [("media.role".into(), "Communication".into())]);
        assert!(rules[1].sink == Sink::Game);
        assert!(rules[2].sink == Sink::Global);
    }

    #[test]
    fn test_config_replaces_defaults() {
        let rules = parse(
            r#"
            [[route]]
            sink = "chat"
            match = [{ "application.name" = "Firefox", "media.name" = "~.*call.*" }]
            "#,
        )
        .unwrap();
        assert!(render(&rules, &Names::default()).starts_with(
            "# Written by novamix-daemon from its config; edits are overwritten.\n\
             novamix.rules = [\n  {\n    matches = [\n      \
             { media.class = \"Stream/Output/Audio\", \"application.name\" = \"Firefox\", \
             \"media.name\" = \"~.*call.*\" }\n    ]\n    \
             actions = { novamix-route = \"input.CHAT\" }\n  }\n]\n"
        ));
    }

    #[test]
    fn test_matching() {
        let rules: Vec<novamix::ipc::Rule> =
            parse(DEFAULTS).unwrap().iter().map(Into::into).collect();
        let props = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<_, _>>()
        };
        assert!(matching(&rules, &props(&[("application.name", "Discord")])) == Some(Sink::Chat));
        assert!(matching(&rules, &props(&[("media.role", "Game")])) == Some(Sink::Game));
        assert!(matching(&rules, &props(&[("application.name", "game.exe")])) == Some(Sink::Game));
        assert!(
            matching(
                &rules,
                &props(&[("application.name", "Music Player Daemon")])
            ) == Some(Sink::Global)
        );
        assert!(matching(&rules, &props(&[("media.role", "Music")])) == Some(Sink::Global));
        assert!(matching(&rules, &props(&[("application.name", "Firefox")])).is_none());
    }

    #[test]
    fn test_empty_config_is_no_rules() {
        assert!(parse("").unwrap().is_empty());
    }

    #[test]
    fn test_write_dial() {
        assert!(write_dial("").unwrap());
        assert!(!write_dial("write_dial = false").unwrap());
        assert!(write_dial("write_dial = 1").is_err());
    }

    #[test]
    fn test_take_default() {
        assert!(!take_default("").unwrap());
        assert!(take_default("take_default = true").unwrap());
        assert!(take_default("take_default = \"yes\"").is_err());
    }

    #[test]
    fn test_bad_sink() {
        assert!(parse("[[route]]\nsink = \"music\"\nmatch = []").is_err());
    }

    #[test]
    fn test_render_escapes() {
        let rules = parse(DEFAULTS).unwrap();
        assert!(render(&rules, &Names::default()).contains(r#""~\\.(exe|EXE)$""#));
    }
}
