extern crate log;
extern crate pretty_env_logger;
extern crate core;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod data;
mod localisation;
mod collection;
mod tech_tree;

use itertools::Itertools;
use rayon::prelude::*;

use tokio_stream::StreamExt;
use std::collections::{BTreeMap, HashMap};
use std::fs::{read_dir, DirEntry};
use std::path::{Path, PathBuf};

use std::{fs, io};
use std::rc::Rc;

use crate::data::{StringOrStruct, Technology, TechnologyData, TechnologyNode};
use crate::localisation::{fold_localisation_map, Languages, Text, read_localisations, Token};
use jomini::{JominiDeserialize, TextDeserializer};
use log::info;
use logos::Lexer;
use measure_time::trace_time;
use regex::Regex;
use serde::Serialize;
use tokio_stream::wrappers::ReadDirStream;
use crate::tech_tree::TechnologyTree;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const WORKSHOP_PATH: &str = "D:\\SteamLibrary\\steamapps\\workshop\\content\\281990";

#[derive(PartialEq, Debug, Serialize)]
pub struct Mod {
    path: PathBuf,
    descriptor: ModDescriptor,
    variables: BTreeMap<String, String>,
    technologies: HashMap<String, TechnologyData>,
    localisations: Vec<(Languages, BTreeMap<String, String>)>,
}

#[derive(JominiDeserialize, PartialEq, Debug, Serialize)]
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

fn read_variables<P: AsRef<Path>>(path: P) -> io::Result<BTreeMap<String, String>> {
    Ok(path
        .as_ref()
        .join("common")
        .join("scripted_variables")
        .read_dir()?
        .filter_map(|x| {
            x.ok().filter(|x| {
                x.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map_or(false, |s| s == "txt")
            })
        })
        .collect::<Vec<DirEntry>>()
        .into_par_iter()
        .map(|x| {
            Ok(TextDeserializer::from_windows1252_slice::<
                BTreeMap<String, String>,
            >(fs::read(x.path())?.as_slice())
                .expect(&format!("Parse mod variables failed, {:?}", x.path())))
        })
        .filter_map(|x: io::Result<BTreeMap<String, String>> | match x {
            Ok(x) => Some(x),
            Err(e) => {
                eprintln!("I/O failed, {:#?}", e);
                None
            }
        })
        .flatten()
        .collect()
    )
}

fn read_technologies<P: AsRef<Path>>(path: P) -> io::Result<(BTreeMap<String, String>, HashMap<String, TechnologyData>)> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let technologies = path
        .as_ref()
        .join("common")
        .join("technology")
        .read_dir()?
        .filter_map(|x| {
            x.ok().filter(|x| {
                x.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map_or(false, |s| s == "txt")
            })
        })
        .collect::<Vec<DirEntry>>()
        .into_par_iter()
        .map(|x| {
            Ok(TextDeserializer::from_windows1252_slice::<
                HashMap<String, StringOrStruct<TechnologyData>>,
            >(fs::read(x.path())?.as_slice())
                .expect(&format!("Parse mod technology failed, {:?}", x.path()))
                .into_iter()
                .filter_map(|(k, v)| match v {
                    StringOrStruct::Str(value) => {
                        tx.clone().send((k, value)).expect("Cannot send");
                        None
                    },
                    StringOrStruct::Object(obj) => Some((k, obj)),
                })
                .collect())
        })
        .filter_map(|x: io::Result<HashMap<String, TechnologyData>>| match x {
            Ok(x) => Some(x),
            Err(e) => {
                eprintln!("I/O failed, {:#?}", e);
                None
            }
        })
        .flatten()
        .collect();

    drop(tx); // Disconnect

    Ok((rx.into_iter().collect(), technologies))
}

async fn read_mods<P: AsRef<Path>>(path: P) -> Result<Vec<Mod>, Box<dyn std::error::Error>> {
    let mod_paths = ReadDirStream::new(tokio::fs::read_dir(path).await?)
        .filter(|x| x.as_ref().map_or(false, |entry| entry.path().is_dir()))
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
        .collect::<Vec<tokio::fs::DirEntry>>()
        .await;

    let descriptors = tokio_stream::iter(mod_paths.into_iter())
        .then(|x| async move {
            let path = x.path();

            let descriptor: ModDescriptor = {
                //trace_time!("Parsing descriptor for {:?}", path);
                TextDeserializer::from_utf8_slice(fs::read(path.join("descriptor.mod"))?.as_slice())
                    .expect("Parse mod descriptor failed")
            };

            let localisations = {
                //trace_time!("Parsing localisations for {:?}", path);
                read_localisations(&path).unwrap_or_default()
            };

            let mut scripted_variables = read_variables(&path).unwrap_or_default();

            let (mut tech_variables, technologies) = {
                //trace_time!("Parsing technologies for {:?}", path);
                read_technologies(&path).unwrap_or_default()
            };

            scripted_variables.append(&mut tech_variables);

            Ok(Mod {
                path,
                variables: scripted_variables,
                technologies,
                descriptor,
                localisations,
            })
        })
        .filter_map(|x: Result<Mod, io::Error>| x.ok())
        .collect()
        .await;
    Ok(descriptors)
}

fn parse_game_files<P: AsRef<Path>>(path: P) -> io::Result<Mod> {
    let path = path.as_ref();

    let version = serde_json::from_str::<serde_json::Value>(&fs::read_to_string(path.join("launcher-settings.json"))?)
        .ok()
        .and_then(|x| x.get("rawVersion").and_then(|y| y.as_str()).map(|y| y.to_string()));

    let descriptor = ModDescriptor {
        name: "Stellaris".to_string(),
        tags: vec![],
        version,
        dependencies: None,
        picture: None,
        supported_version: None,
        remote_file_id: "Stellaris".to_string(),
    };

    let localisations = {
        //trace_time!("Parsing localisations for {:?}", path);
        read_localisations(&path)?
    };

    let mut scripted_variables = read_variables(&path)?;

    let (mut tech_variables, technologies) = {
        //trace_time!("Parsing technologies for {:?}", path);
        read_technologies(&path)?
    };

    scripted_variables.append(&mut tech_variables);

    Ok(Mod {
        path: path.to_path_buf(),
        technologies,
        variables: scripted_variables,
        descriptor,
        localisations,
    })
}

trait ReplaceVariables<'a> {
    fn replace_variables<F: FnMut(&'a str) -> &str>(&'a self, f: F) -> String;
}

impl<'a> ReplaceVariables<'a> for &'a str {
    fn replace_variables<F: FnMut(&'a str) -> &str>(&'a self, f: F) -> String {
        let mut ret = String::new();
        self.split('$');
        todo!()
    }
}

impl<'a> ReplaceVariables<'a> for String {
    fn replace_variables<F: FnMut(&'a str) -> &str>(&'a self, f: F) -> String {
        todo!()
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use logos::Logos;
    let mut lexer: Lexer<Token> = Token::lexer("§Y$matter_decompressor_4$§!扭曲黑洞的引力，形成引力钻头，从奇点中获取£minerals£矿物。");
    println!("{:#?}", lexer.collect::<Vec<Token>>());

    //pretty_env_logger::init();
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    info!("Stellaris Tech Tree Parser {}", VERSION);

    let mut mods = {
        trace_time!("Parse all mods");
        read_mods(WORKSHOP_PATH).await?
    };

    mods.push(parse_game_files(r#"D:\SteamLibrary\steamapps\common\Stellaris"#)?);

    let mods = mods;

    let all_localisations: HashMap<Languages, BTreeMap<&str, &str>> = {
        trace_time!("Parse all localisations");
        mods.iter()
        .flat_map(|x| x.localisations.iter().map(|(k, v)| (*k, v)))
        .into_grouping_map()
        .fold(BTreeMap::new(), |mut acc, _key, val| {
            val.iter().for_each(|(k, v)| {
                acc.insert(k.as_str(), v.as_str());
            });
            acc
        })
    };

    let all_variables: BTreeMap<&String, &String> = {
        trace_time!("Parse all variables");
        mods
            .par_iter()
            .flat_map(|x| x.variables.par_iter())
            .collect()
    };

    let regex = Regex::new("\\$(\\w+)\\$").unwrap();

    let all_localisations: HashMap<Languages, BTreeMap<&str, String>> = {
        trace_time!("Replace variables");
        all_localisations
            .into_iter()
            .map(|(lang, map)| {
                let map = map
                    .par_iter()
                    .map(|(key, value)| {
                        let mut ret = value.to_string();
                        for captures in regex.captures_iter(&*value) {
                            if let Some(variable) = captures.get(1) {
                                let variable = variable.as_str();
                                // TODO Replace more variables
                                if let Some(variable_value) = map.get(variable) {
                                    ret = ret.replacen(&format!("${}$", variable), variable_value, 1);
                                }
                            }
                        }
                        (*key, ret)
                    })
                    .collect();
                (lang, map)
            })
            .collect()
    };

    let folded_localisations: HashMap<Languages, BTreeMap<String, Text>> = {
        trace_time!("Fold localisations");
        all_localisations
            .par_iter()
            .map(|(key, value)| (*key, fold_localisation_map(&value)))
            .collect()
    };

    {
        trace_time!("Write localisations");
        tokio::fs::write(
            format!("mods/localisation.json"),
            simd_json::to_string_pretty(&folded_localisations)?,
        ).await?;
    }

    let all_technologies: Vec<Technology> = {
        trace_time!("Fold technologies");
        mods.par_iter().flat_map(|x| {
            let data = x;
            data
                .technologies
                .par_iter()
                .map(|(name, tech_data)| {
                    let localisation = folded_localisations
                        .iter()
                        .filter_map(|(lang, map)| Some((lang.clone(), map.get(name)?.clone())))
                        .collect();

                    Technology {
                        modid: data.descriptor.remote_file_id.to_string(),
                        name: name.to_string(),
                        id: name.to_string(), // TODO
                        localisation,

                        cost: tech_data.cost.as_ref().map(|cost|
                            if let Some(replaced) = all_variables.get(&cost) {
                                replaced.to_string()
                            } else {
                                cost.clone()
                            }
                        ).and_then(|cost| cost.parse().ok()).unwrap_or_default(),

                        tier: tech_data.tier.as_ref().map(|tier|
                            if let Some(replaced) = all_variables.get(&tier) {
                                replaced.to_string()
                            } else {
                                tier.clone()
                            }
                        ),
                        category: tech_data.category.first().cloned(),
                        weight: tech_data.weight.first().as_ref().map(|cost|
                            if let Some(replaced) = all_variables.get(*cost) {
                                replaced.to_string()
                            } else {
                                cost.to_string()
                            }
                        ),
                        area: tech_data.area.clone(),
                        prerequisites: tech_data.prerequisites.clone(),
                        start_tech: tech_data.start_tech,
                    }
                })
        }).collect()
    };

    let technologies_map: HashMap<&str, Rc<Technology>> = all_technologies.iter().map(|x| (x.id.as_str(), Rc::new(x.clone()))).collect();

    let mut tech_tree = TechnologyTree::default();

    tech_tree.insert_map(&technologies_map);

    std::fs::write("tech_tree.txt", format!("{:#?}", tech_tree))?;

    let technologies_map: HashMap<&str, TechnologyNode> = technologies_map.iter().map(|(id, tech)| {
        (*id, TechnologyNode {
            id: id.to_string(),
            data: tech.clone(),
            prerequisites: tech.prerequisites.iter().filter_map(|x| technologies_map.get(x.as_str())).map(|x| x.clone()).collect()
        })
    }).collect();



    {
        trace_time!("Write technologies");
        tokio::fs::write(
            format!("mods/all_technologies.json"),
            simd_json::to_string_pretty(&all_technologies)?,
        ).await?;
    }

    {
        trace_time!("Write technologies map");
        tokio::fs::write(
            format!("mods/technologies_map.json"),
            simd_json::to_string_pretty(&technologies_map)?,
        ).await?;
    }
    Ok(())
}
