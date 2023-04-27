use anyhow::anyhow;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct IronyModCollection {
    #[serde(alias = "Game")]
    pub game: String,

    #[serde(alias = "Name")]
    pub name: String,

    #[serde(alias = "IsSelected")]
    pub is_selected: bool,

    #[serde(alias = "Mods")]
    pub mod_registry_ids: Vec<String>,
}

impl IronyModCollection {
    pub fn get_mod_ids(&self) -> Vec<u64> {
        self.mod_registry_ids
            .iter()
            .map(|x| x.trim_start_matches("mod/ugc_").trim_end_matches(".mod"))
            .filter_map(|x| x.parse::<u64>().ok())
            .collect()
    }
}

impl Display for IronyModCollection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

pub fn parse_irony_collections<P: AsRef<Path>>(
    path: P,
) -> anyhow::Result<(Option<PathBuf>, Vec<IronyModCollection>)> {
    let path = path.as_ref();

    let values: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(path)?)?;

    let workshop_path = values
        .iter()
        .find(|x| {
            x["Name"]
                .as_str()
                .map(|n| n == "GameSettings")
                .unwrap_or_default()
        })
        .and_then(|x| {
            x["Value"]
                .as_array()
                .and_then(|v| {
                    v.iter().find(|z| {
                        z["Type"]
                            .as_str()
                            .map(|n| n == "Stellaris")
                            .unwrap_or_default()
                    })
                })
                .and_then(|v| v["ExecutableLocation"].as_str())
                .map(|v| Path::new(v))
                .and_then(|v| v.parent())
                .and_then(|v| v.parent())
                .and_then(|v| v.parent())
                .map(|v| {
                    v.to_path_buf()
                        .join("workshop")
                        .join("content")
                        .join("281990")
                })
        });

    Ok((
        workshop_path,
        values
            .into_iter()
            .find(|x| {
                x["Name"]
                    .as_str()
                    .map(|n| n == "ModCollection")
                    .unwrap_or_default()
            })
            .and_then(|x| {
                x["Value"].as_array().map(|v| {
                    v.into_iter()
                        .filter_map(|y| {
                            serde_json::from_value::<IronyModCollection>(y.clone()).ok()
                        })
                        .filter(|y| y.game == "Stellaris")
                        .collect()
                })
            })
            .unwrap_or_default(),
    ))
}

pub fn parse_paradox_launcher_registry<P: AsRef<Path>>(
    path: P,
) -> anyhow::Result<HashMap<String, String>> {
    let path = path.as_ref();

    let value: Value = serde_json::from_slice(&std::fs::read(path)?)?;

    let obj = value
        .as_object()
        .ok_or(anyhow!("mods_registry.json broken"))?;

    Ok(obj
        .into_iter()
        .map(|(id, x)| {
            (
                id.to_string(),
                x["dirPath"]
                    .as_str()
                    .expect("mods_registry.json broken")
                    .to_string(),
            )
        })
        .collect())
}

pub fn parse_paradox_launcher_load_order<P: AsRef<Path>>(
    path: P,
    registry: &HashMap<String, String>,
) -> anyhow::Result<Vec<String>> {
    let path = path.as_ref();

    let value: Value = serde_json::from_slice(&std::fs::read(path)?)?;

    let order = value["modsOrder"]
        .as_array()
        .ok_or(anyhow!("game_data.json broken"))?;

    Ok(order
        .iter()
        .filter_map(|x| x.as_str())
        .filter_map(|x| registry.get(x).cloned())
        .collect())
}
