use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct IronyModCollection {
    #[serde(alias = "Game")]
    pub game: String,

    #[serde(alias = "Name")]
    pub name: String,

    #[serde(alias = "IsSelected")]
    pub is_selected: bool,

    #[serde(alias = "Mods")]
    mods: Vec<String>,
}

impl IronyModCollection {
    pub fn get_mod_ids(&self) -> Vec<u64> {
        self.mods.iter()
            .map(|x| x.trim_start_matches("mod/ugc_").trim_end_matches(".mod"))
            .filter_map(|x| x.parse::<u64>().ok())
            .collect()
    }
}

pub fn parse_irony_collections() -> anyhow::Result<()> {
    let user_folder = std::env::var("USERPROFILE")?;

    Ok(())
}