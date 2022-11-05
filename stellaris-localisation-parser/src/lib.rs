use std::collections::{BTreeMap, BTreeSet};
use std::process::Output;

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

#[derive(PartialEq, Hash, Clone, Default, Debug, Serialize, Deserialize)]
pub struct Text {
    pub value: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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

pub fn parse_localisation<I: Iterator<Item = String>>(mut lines: I) -> anyhow::Result<(Languages, BTreeMap<String, String>)> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();

    let mut current_language = Languages::Default;

    let mut i = 0;
    while let Some(line) = lines.next() {
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
