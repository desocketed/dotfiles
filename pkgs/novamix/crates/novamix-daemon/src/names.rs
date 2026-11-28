//! The sinks' names, from the `[sinks]` table of the config. The names drive the node names
//! and descriptions; the keywords `game`, `chat` and `global` of the CLI and of `[[route]]` stay.

use anyhow::anyhow;
use novamix::Result;
use novamix::mix::Sink;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Names {
    pub game: String,
    pub chat: String,
    pub global: String,
}
impl Default for Names {
    fn default() -> Self {
        Self {
            game: "GAME".into(),
            chat: "CHAT".into(),
            global: "GLOBAL".into(),
        }
    }
}
impl Names {
    /// `[sinks]` with `game`, `chat` and `global`, each optional.
    pub fn parse(text: &str) -> Result<Self> {
        let table: toml::Table = text.parse().or(Err(anyhow!("Config is not valid TOML")))?;
        let mut names = Self::default();
        let Some(sinks) = table.get("sinks") else {
            return Ok(names);
        };
        let sinks = sinks
            .as_table()
            .ok_or(anyhow!("sinks must be a [sinks] table"))?;
        for (key, value) in sinks {
            let name = value
                .as_str()
                .filter(|name| !name.is_empty() && !name.contains(char::is_whitespace))
                .ok_or(anyhow!(
                    "sinks.{key} must be a name without spaces, like \"{}\"",
                    key.to_uppercase()
                ))?;
            match key.as_str() {
                "game" => names.game = name.into(),
                "chat" => names.chat = name.into(),
                "global" => names.global = name.into(),
                _ => return Err(anyhow!("Unknown sink {key} in [sinks]")),
            }
        }
        if names.game == names.chat || names.game == names.global || names.chat == names.global {
            return Err(anyhow!("The sinks need different names"));
        }
        Ok(names)
    }

    pub fn name(
        &self,
        sink: Sink,
    ) -> &str {
        match sink {
            Sink::Game => &self.game,
            Sink::Chat => &self.chat,
            Sink::Global => &self.global,
        }
    }

    /// `pw-loopback -n X` names its sink node `input.X`.
    pub fn node_name(
        &self,
        sink: Sink,
    ) -> String {
        format!("input.{}", self.name(sink))
    }

    /// The sink a node is, by its `node.name`.
    pub fn sink_of(
        &self,
        node_name: &str,
    ) -> Option<Sink> {
        Sink::ALL
            .into_iter()
            .find(|sink| self.node_name(*sink) == node_name)
    }

    /// The loopbacks' own playback sides, `output.X`: streams, but not the user's.
    pub fn is_output(
        &self,
        node_name: &str,
    ) -> bool {
        Sink::ALL
            .into_iter()
            .any(|sink| node_name == format!("output.{}", self.name(sink)))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_defaults_without_section() {
        let names = Names::parse("write_dial = false").unwrap();
        assert!(names == Names::default());
        assert!(names.node_name(Sink::Game) == "input.GAME");
        assert!(names.node_name(Sink::Global) == "input.GLOBAL");
    }

    #[test]
    fn test_renamed() {
        let names = Names::parse("[sinks]\nchat = \"Voice\"").unwrap();
        assert!(names.node_name(Sink::Chat) == "input.Voice");
        assert!(names.node_name(Sink::Game) == "input.GAME");
        assert!(names.sink_of("input.Voice") == Some(Sink::Chat));
        assert!(names.sink_of("input.CHAT").is_none());
        assert!(names.is_output("output.Voice"));
    }

    #[test]
    fn test_bad_names() {
        assert!(Names::parse("[sinks]\nmusic = \"Music\"").is_err());
        assert!(Names::parse("[sinks]\ngame = \"\"").is_err());
        assert!(Names::parse("[sinks]\ngame = \"My Game\"").is_err());
        assert!(Names::parse("[sinks]\ngame = \"X\"\nchat = \"X\"").is_err());
        assert!(Names::parse("[sinks]\nglobal = \"CHAT\"").is_err());
        assert!(Names::parse("sinks = 3").is_err());
    }
}
