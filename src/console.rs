use std::ffi::OsString;
use std::path::Path;
use anyhow::anyhow;
use inquire::error::InquireResult;
use inquire::{Confirm, Select};
use itertools::Itertools;
use crate::collection::{parse_irony_collections, parse_paradox_launcher_load_order, parse_paradox_launcher_registry};

pub fn query() -> anyhow::Result<Vec<String>> {
    let user_folder = std::env::var("USERPROFILE")?;
    let app_data = std::env::var("APPDATA")?;

    let irony_path = Path::new(&app_data).join("Mario").join("IronyModManager");

    if irony_path.is_dir() {
        println!("Found Irony Mod Manager data folder in {}", irony_path.display());
        if Confirm::new("Use Irony Mod Manager?").with_default(true).prompt()? {

            let files = std::fs::read_dir(&irony_path)?.filter_map(|x| x.ok()).map(|x| x.file_name().to_string_lossy().to_string()).sorted().rev().collect::<Vec<_>>();

            let irony_db = Select::new("Select a Irony user database", files).prompt()?;

            let (workshop_path, collections) = parse_irony_collections(irony_path.join(irony_db))?;

            let irony_db = Select::new("Select a Stellaris mod collection", collections.clone())
                .with_starting_cursor(collections.iter().find_position(|x| x.is_selected == true).map(|(i, _)| i).unwrap_or_default())
                .prompt()?;

            return if let Some(workshop_path) = workshop_path {
                println!("Workshop path: {:?}", workshop_path);
                println!("Parsing Irony collection: {} with {} mods", &irony_db.name, irony_db.get_mod_ids().len());

                Ok(irony_db.get_mod_ids().iter().map(|x| workshop_path.join(x.to_string()).to_string_lossy().to_string()).collect())
            } else {
                Err(anyhow!("Cannot resolve workshop path"))
            }
        }
    }

    let game_data_path = Path::new(&user_folder).join("Documents").join("Paradox Interactive").join("Stellaris");

    if game_data_path.is_dir() {
        println!("Found Paradox launcher data folder in {}", game_data_path.display());

        let ans = Confirm::new("Do you want to use Paradox Launcher's current load order?").with_default(true).prompt()?;

        let registry = parse_paradox_launcher_registry(game_data_path.join("mods_registry.json"))?;
        let collection = parse_paradox_launcher_load_order(game_data_path.join("game_data.json"), &registry)?;

        println!("Parsing Paradox launcher collection with {} mods", collection.len());
        return Ok(collection);
    }

    Err(anyhow!("Not selecting any collection"))
}