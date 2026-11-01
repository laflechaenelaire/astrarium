use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Es,
}

#[derive(Default, Serialize, Deserialize)]
struct Preferences {
    #[serde(default)]
    language: Language,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

impl Language {
    pub fn pick<'a>(self, en: &'a str, es: &'a str) -> &'a str {
        match self {
            Self::En => en,
            Self::Es => es,
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::En => Self::Es,
            Self::Es => Self::En,
        }
    }

    fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("astrarium/preferences.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<Preferences>(&s).ok())
            .unwrap_or_default()
            .language
    }

    pub fn save(self) -> io::Result<()> {
        let path =
            Self::path().ok_or_else(|| io::Error::other("Configuration directory unavailable"))?;
        let mut preferences = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Preferences>(&s).ok())
            .unwrap_or_default();
        preferences.language = self;
        fs::create_dir_all(path.parent().unwrap())?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(&preferences)?)?;
        fs::rename(temporary, path)
    }

    pub fn direction(self, az: f64) -> &'static str {
        let directions = match self {
            Self::En => ["N", "NE", "E", "SE", "S", "SW", "W", "NW"],
            Self::Es => ["N", "NE", "E", "SE", "S", "SO", "O", "NO"],
        };
        directions[((az.rem_euclid(360.0) / 45.0).round() as usize) % 8]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_english_and_preserves_other_preferences() {
        assert_eq!(Language::default(), Language::En);
        let p: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(p.language, Language::En);
        let mut p: Preferences = serde_json::from_str(r#"{"language":"es","future":42}"#).unwrap();
        assert_eq!(p.language, Language::Es);
        p.language = Language::En;
        let json = serde_json::to_value(p).unwrap();
        assert_eq!(json["language"], "en");
        assert_eq!(json["future"], 42);
        assert_eq!(Language::En.direction(270.0), "W");
        assert_eq!(Language::Es.direction(270.0), "O");
    }
}
