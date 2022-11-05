use crate::Serialize;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::BufRead;
use std::path::Path;
use std::{fs, io};
use std::str::FromStr;
use anyhow::anyhow;
use log::warn;
use measure_time::trace_time;
use memmap2::Mmap;
use rayon::prelude::*;
use regex::Regex;
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};
use trim_in_place::TrimInPlace;
use walkdir::WalkDir;
use logos_derive::Logos;

#[derive(PartialEq, Eq, Debug, Hash, Copy, Clone, Serialize, EnumString, Display, IntoStaticStr, EnumIter)]
pub enum Languages {
    #[strum(serialize = "braz_por")]
    Portuguese,

    #[strum(serialize = "english")]
    English,

    #[strum(serialize = "french")]
    French,

    #[strum(serialize = "german")]
    German,

    #[strum(serialize = "polish")]
    Polish,

    #[strum(serialize = "russian")]
    Russian,

    #[strum(serialize = "spanish")]
    Spanish,

    #[strum(serialize = "simp_chinese")]
    SimplifiedChinese,

    #[strum(serialize = "default")]
    Default,
}

impl Default for Languages {
    fn default() -> Self {
        Languages::Default
    }
}

pub fn fold_localisation_map(map: &BTreeMap<&str, String>) -> BTreeMap<String, Text> {

    let keys: BTreeSet<&str> = {
        trace_time!("Mapping localisation keys");
        map
            .keys()
            .map(|x| {
                x.trim_end_matches("name")
                    .trim_end_matches("desc")
                    .trim_end_matches(&[':', '_', '.'])
            })
            .collect()
    };

    let mut ret = BTreeMap::new();

    for key in keys {
        let name = map
            .get(format!("{}_name", key).as_str())
            .or(map.get(format!("{}.name", key).as_str()))
            .map(|x| x.trim_matches('"'));
        let desc = map
            .get(format!("{}_desc", key).as_str())
            .or(map.get(format!("{}.desc", key).as_str()))
            .map(|x| x.trim_matches('"'));
        let value = map
            .get(key)
            .map(|x| x.trim_matches('"'))
            .or(name.clone())
            .or(desc.clone());
        ret.insert(
            key.to_string(),
            Text {
                value: value.map(|x| x.to_string()).unwrap_or_default(),

                name: name.map(|x| x.to_string()),
                description: desc.map(|x| x.to_string()),
            },
        );
    }

    ret
}

#[derive(PartialEq, Hash, Clone, Default, Debug, Serialize, Deserialize, Eq)]
pub struct Text {
    pub value: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Logos, Debug, PartialEq)]
pub enum Token<'a> {
    /// §!: Return to color before last color change
    #[token("§!")]
    ColorEnd,

    #[regex("\\$\\w+\\$")]
    Variable(&'a str),

    /// §W §T §L §P §R §S §H §Y §G §E §B §M
    ///
    /// See: https://stellaris.paradoxwikis.com/Localisation_modding#Color_Codes
    #[regex("§[WTLPRSHYGEBM]", |lex| ColorCode::from(lex.slice()))]
    ColorCode(ColorCode),

    #[regex(".+")]
    Text(&'a str),

    #[error]
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorCode {
    White,
    LightGrey,
    Brown,
    LightRed,
    Red,
    DarkOrange,
    Orange,
    Yellow,
    Green,
    Teal,
    Blue,
    Purple,
    Default
}

impl From<char> for ColorCode {
    fn from(value: char) -> Self {
        match value {
            'W' => ColorCode::White,
            'T' => ColorCode::LightGrey,
            'L' => ColorCode::Brown,
            'P' => ColorCode::LightRed,
            'R' => ColorCode::Red,
            'S' => ColorCode::DarkOrange,
            'H' => ColorCode::Orange,
            'Y' => ColorCode::Yellow,
            'G' => ColorCode::Green,
            'E' => ColorCode::Teal,
            'B' => ColorCode::Blue,
            'M' => ColorCode::Purple,
            _ => ColorCode::Default
        }
    }
}

impl From<ColorCode> for char {
    fn from(value: ColorCode) -> Self {
        match value {
            ColorCode::White => 'W',
            ColorCode::LightGrey => 'T',
            ColorCode::Brown => 'L',
            ColorCode::LightRed => 'P',
            ColorCode::Red => 'R',
            ColorCode::DarkOrange => 'S',
            ColorCode::Orange => 'H',
            ColorCode::Yellow => 'Y',
            ColorCode::Green => 'G',
            ColorCode::Teal => 'E',
            ColorCode::Blue => 'B',
            ColorCode::Purple => 'M',
            ColorCode::Default => '!',
        }
    }
}

impl FromStr for ColorCode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "§W" => Ok(ColorCode::White),
            "§T" => Ok(ColorCode::LightGrey),
            "§L" => Ok(ColorCode::Brown),
            "§P" => Ok(ColorCode::LightRed),
            "§R" => Ok(ColorCode::Red),
            "§S" => Ok(ColorCode::DarkOrange),
            "§H" => Ok(ColorCode::Orange),
            "§Y" => Ok(ColorCode::Yellow),
            "§G" => Ok(ColorCode::Green),
            "§E" => Ok(ColorCode::Teal),
            "§B" => Ok(ColorCode::Blue),
            "§M" => Ok(ColorCode::Purple),
            "§!" => Ok(ColorCode::Default),
            _ => Err(anyhow!("Not a valid color code"))
        }
    }
}

impl From<&str> for ColorCode {
    fn from(value: &str) -> Self {
        match value {
            "§W" => ColorCode::White,
            "§T" => ColorCode::LightGrey,
            "§L" => ColorCode::Brown,
            "§P" => ColorCode::LightRed,
            "§R" => ColorCode::Red,
            "§S" => ColorCode::DarkOrange,
            "§H" => ColorCode::Orange,
            "§Y" => ColorCode::Yellow,
            "§G" => ColorCode::Green,
            "§E" => ColorCode::Teal,
            "§B" => ColorCode::Blue,
            "§M" => ColorCode::Purple,
            _ => ColorCode::Default,
        }
    }
}

impl From<ColorCode> for &str {
    fn from(value: ColorCode) -> Self {
        match value {
            ColorCode::White => "§W",
            ColorCode::LightGrey => "§T",
            ColorCode::Brown => "§L",
            ColorCode::LightRed => "§P",
            ColorCode::Red => "§R",
            ColorCode::DarkOrange => "§S",
            ColorCode::Orange => "§H",
            ColorCode::Yellow => "§Y",
            ColorCode::Green => "§G",
            ColorCode::Teal => "§E",
            ColorCode::Blue => "§B",
            ColorCode::Purple => "§M",
            ColorCode::Default => "§!",
        }
    }
}

#[derive(PartialEq)]
enum State {
    InitOrEnd,
    ColorStart
}

pub fn parse_localisation_token(s: &str) {
    let mut state = State::InitOrEnd;

    let mut vec: Vec<char> = Vec::new();


    for ch in s.chars() {
        match ch {
            '§' => {
                if state == State::ColorStart {
                    warn!("Localisation {} contains §§, report to its author.", s);
                }

            }
            _ => {
                match state {
                    State::InitOrEnd => {
                        vec.push(ch);
                    }
                    State::ColorStart => {
                        match ch {
                            'W' | 'T' | 'L' | 'P' | 'R' | 'S' | 'H' | 'Y' | 'G' | 'E' | 'B' | 'M' => {

                            }
                            _ => {
                                warn!("Not a valid color code: {}", ch);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn read_localisations<P: AsRef<Path>>(
    path: P,
) -> io::Result<Vec<(Languages, BTreeMap<String, String>)>> {
    Ok(WalkDir::new(path.as_ref().join("localisation"))
        .into_iter()
        .filter_map(|x| {
            x.ok().filter(|x| {
                x.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map_or(false, |s| s == "yml")
            })
        })
        .collect::<Vec<walkdir::DirEntry>>()
        .into_par_iter()
        .map(|x| Ok(parse_localisation(x.path())))
        .filter_map(
            |x: Result<anyhow::Result<(Languages, BTreeMap<String, String>)>, io::Error>| match x {
                Ok(x) => match x {
                    Ok(x) => Some(x),
                    Err(e) => {
                        eprintln!("Localization parsing failed, {:#?}", e);
                        None
                    }
                },
                Err(e) => {
                    eprintln!("Localization parsing failed, {:#?}", e);
                    None
                }
            },
        )
        //.filter_map(|x: Option<(Languages, BTreeMap<String, String>)>| x)
        .collect())
}

pub async fn parse_localisation_async(path: &Path) -> anyhow::Result<(Languages, BTreeMap<String, String>)> {
    use tokio::io::AsyncBufReadExt;
    let mut map: BTreeMap<String, String> = BTreeMap::new();

    let file = tokio::fs::OpenOptions::new()
        .read(true)
        .open(path)
        .await
        .expect("opening translation failed");
    let file = tokio::io::BufReader::new(file);
    let mut lines_iterator = file.lines();

    let mut current_language = Languages::Default;

    while let Ok(Some(line)) = lines_iterator.next_line().await {
        let mut line = line;

        line.trim_in_place();
        if let Some(comment_index) = line.find('#') {
            line = (&line[0..comment_index]).to_string();
        }
        if line.is_empty() {
            continue;
        }
        if line.starts_with("\u{feff}") {
            line.drain(..3); // Remove BOM
        }

        if let Some(collon) = line.find(':') {
            let pair = line.split_at(collon);
            let key = pair.0.to_owned();
            let value = pair.1.to_owned();
            if key.starts_with("l_") && current_language == Languages::Default {
                current_language = key.trim_start_matches("l_").parse::<Languages>().unwrap_or_default();
            }
            if let Some(left) = value.find('"') {
                if let Some(right) = value.rfind('"') {
                    map.insert(
                        key.to_string(),
                        (&value[left..right]).trim_matches('"').to_string(),
                    );
                }
            }
        }
    }

    Ok((current_language, map))
}

pub fn parse_localisation(path: &Path) -> anyhow::Result<(Languages, BTreeMap<String, String>)> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();

    let file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .expect("opening translation failed");
    let mmap = unsafe { Mmap::map(&file) }?;

    let file = io::BufReader::new(mmap.as_ref());
    let mut lines_iterator = file.lines();

    let mut current_language = Languages::Default;

    let mut i = 0;
    while let Some(line) = lines_iterator.next() {
        let mut line_owned = line.expect("There was an error reading a file");

        if i == 0 && line_owned.starts_with("\u{feff}") {
            line_owned.drain(..3); // Remove BOM
        }

        line_owned.trim_in_place();

        if line_owned.is_empty() {
            continue;
        }

        let mut line = line_owned.as_str();

        if let Some(comment_index) = line_owned.find('#') {
            line = &line_owned[0..comment_index];
        }

        if let Some(collon) = line.find(':') {
            let (key, value) = line.split_at(collon);
            if current_language == Languages::Default && key.starts_with("l_") {
                current_language = key.trim_start_matches("l_").parse::<Languages>().unwrap_or_default();
            }
            if let Some(left) = value.find('"') {
                if let Some(right) = value.rfind('"') {
                    map.insert(
                        key.to_string(),
                        (&value[left..right]).trim_matches('"').to_string(),
                    );
                }
            }
        }

        i += 1;
    }

    Ok((current_language, map))
}
