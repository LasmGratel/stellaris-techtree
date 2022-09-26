mod localisation;
mod data;

use std::fs::{DirEntry, read_dir};
use std::{fs, io};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use rayon::prelude::*;

use jomini::{JominiDeserialize, TextDeserializer, TextTape};
use strip_bom::StripBom;
use crate::data::{deserialize_technologies, StringOrStruct, Technologies, Technology};

const WORKSHOP_PATH: &str = "D:\\SteamLibrary\\steamapps\\workshop\\content\\281990";

#[derive(PartialEq, Eq, Debug, Hash, Copy, Clone)]
enum Languages {
    Portuguese,
    English,
    French,
    German,
    Polish,
    Russian,
    Spanish,
    SimplifiedChinese,
    Default
}

impl Into<String> for Languages {
    fn into(self) -> String {
        match self {
            Languages::Portuguese => "braz_por",
            Languages::English => "english",
            Languages::French => "french",
            Languages::German => "german",
            Languages::Polish => "polish",
            Languages::Russian => "russian",
            Languages::Spanish => "spanish",
            Languages::SimplifiedChinese => "simp_chinese",
            Languages::Default => "default",
        }.to_string()
    }
}

impl FromStr for Languages {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl From<&str> for Languages {
    fn from(s: &str) -> Self {
        match s {
            "braz_por" => Languages::Portuguese,
            "english" => Languages::English,
            "french" => Languages::French,
            "german" => Languages::German,
            "polish" => Languages::Polish,
            "russian" => Languages::Russian,
            "spanish" => Languages::Spanish,
            "simp_chinese" => Languages::SimplifiedChinese,
            _ => Languages::Default
        }
    }
}

#[derive(PartialEq, Debug)]
struct Mod {
    path: PathBuf,
    descriptor: ModDescriptor,
    localisations: HashMap<Languages, HashMap<String, String>>
}

#[derive(JominiDeserialize, PartialEq, Debug)]
struct ModDescriptor {
    name: String,


    #[jomini(default)]
    tags: Vec<String>,

    version: Option<String>,
    dependencies: Option<Vec<String>>,
    picture: Option<String>,
    supported_version: Option<String>,

    remote_file_id: String,
}

fn read_localisations<P: AsRef<Path>>(path: P) -> io::Result<HashMap<Languages, HashMap<String, String>>> {
    Ok(path.as_ref().join("localisation")
        .read_dir()?
        .filter_map(|x| x.ok().filter(|x| x.path().extension().and_then(|s| s.to_str()).map_or(false, |s| s == "yml")))
        .collect::<Vec<DirEntry>>()
        .into_par_iter()
        .map(|x| Ok(fs::read_to_string(x.path())?) )
        .map(|x: Result<String, io::Error>| {
            use chumsky::Parser;
            let (parsed, _error) = localisation::parser().parse_recovery(x?.strip_bom());
            Ok(parsed.map(|(language, entries)| {
                (Languages::from_str(&language).unwrap(), entries.into_iter().collect())
            }))
        })
        .filter_map(|x: Result<Option<(Languages, HashMap<String, String>)>, io::Error>| x.ok())
        .filter_map(|x: Option<(Languages, HashMap<String, String>)>| x)
        .collect()
    )
}

fn read_technologies<P: AsRef<Path>>(path: P) -> io::Result<Vec<HashMap<String, Technology>>> {
    Ok(path.as_ref().join("common").join("technology")
        .read_dir()?
        .filter_map(|x| x.ok().filter(|x| x.path().extension().and_then(|s| s.to_str()).map_or(false, |s| s == "txt")))
        .collect::<Vec<DirEntry>>()
        .into_par_iter()
        .map(|x|
            Ok(
                TextDeserializer::from_windows1252_slice::<HashMap<String, StringOrStruct<Technology>>>(fs::read(x.path())?.as_slice())
                    .expect(&format!("Parse mod technology failed, {:?}", x.path()))
                    .into_iter()
                    .filter_map(|(k, v)| match v {
                        StringOrStruct::Str(_) => None,
                        StringOrStruct::Object(obj) => {
                            Some((k, obj))
                        }
                    })
                    .collect()
            )
        )
        .filter_map(|x: io::Result<HashMap<String, Technology>>| x.ok())
        .collect())
}

fn read_mods() -> Result<(), Box<dyn std::error::Error>> {
    let mod_paths = read_dir(WORKSHOP_PATH)?
        .filter(|x| {
            x.as_ref().map_or(false, |entry| entry.path().is_dir())
        })
        .filter(|x| x.is_ok()).map(|x| x.unwrap())
        .collect::<Vec<DirEntry>>();

    let descriptors: Vec<Mod> = mod_paths
        .par_iter()
        .map(|x| {
            let path = x.path();
            let descriptor = TextDeserializer::from_utf8_slice(fs::read(path.join("descriptor.mod"))?.as_slice()).expect("Parse mod descriptor failed");

            let localisations = read_localisations(&path)?;
            let technologies = read_technologies(&path)?;

            Ok(Mod {
                path,
                descriptor,
                localisations,
            })
        })
        .filter_map(|x: Result<Mod, io::Error>| x.ok())
        .collect();

    println!("{:?}", descriptors);
    Ok(())
}

fn read_tech() -> Result<(), Box<dyn std::error::Error>> {
    const TEST_PATH: &str = r#"D:\SteamLibrary\steamapps\workshop\content\281990\2551568342\common\technology\cx_hive_ap_techs.txt"#;
    let map = TextDeserializer::from_windows1252_slice::<HashMap<String, StringOrStruct<Technology>>>(fs::read(TEST_PATH)?.as_slice())?;
    println!("{:#?}", map);
    //deserialize_technologies(TextTape::from_slice(fs::read(TEST_PATH)?.as_slice())?);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //read_tech()?;
    read_mods()?;
    println!("Hello, world!");
    Ok(())
}
