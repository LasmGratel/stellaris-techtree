use crate::localisation::{Languages, Text};
use jomini::{JominiDeserialize, TextTape};
use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::rc::Rc;
use std::str::FromStr;
use anyhow::anyhow;
use jomini::text::Operator;
use serde_json::Value;

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum ResearchArea {
    Society,
    Physics,
    Engineering,
    Anomaly,
    Other(String),
}

impl FromStr for ResearchArea {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "society" => ResearchArea::Society,
            "physics" => ResearchArea::Physics,
            "engineering" => ResearchArea::Engineering,
            "anomaly" => ResearchArea::Anomaly,
            _ => ResearchArea::Other(s.to_string()),
        })
    }
}

impl AsRef<str> for ResearchArea {
    fn as_ref(&self) -> &str {
        match self {
            ResearchArea::Society => "society",
            ResearchArea::Physics => "physics",
            ResearchArea::Engineering => "engineering",
            ResearchArea::Anomaly => "anomaly",
            ResearchArea::Other(s) => s,
        }
    }
}

impl ToString for ResearchArea {
    fn to_string(&self) -> String {
        String::from(self.as_ref())
    }
}

impl Default for ResearchArea {
    fn default() -> Self {
        Self::Other(String::from("unknown"))
    }
}

impl Serialize for ResearchArea {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}

impl<'de> Deserialize<'de> for ResearchArea {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ResearchAreaVisitor;

        impl<'de> Visitor<'de> for ResearchAreaVisitor {
            type Value = ResearchArea;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("Expect a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(ResearchArea::from_str(v).unwrap())
            }
        }

        deserializer.deserialize_str(ResearchAreaVisitor)
    }
}

#[derive(PartialEq, Debug, Default, Clone, Serialize)]
pub struct Modifier {
    pub factor: Option<String>,

    pub not: Option<Vec<Box<Modifier>>>,

    pub or: Option<Vec<Box<Modifier>>>,

    pub nand: Option<Vec<Box<Modifier>>>,

    pub and: Option<Vec<Box<Modifier>>>,

    pub modifiers: HashMap<String, String>
}

impl<'de> Deserialize<'de> for Modifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        struct ModifierVisitor;

        impl<'de> Visitor<'de> for ModifierVisitor {
            type Value = Modifier;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_fmt(format_args!("a modifier"))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
                println!("Visit str: {}", v);
                Err(serde::de::Error::custom("debug"))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
                while let Some(element) = seq.next_element::<Value>()? {
                    println!("{:?}", element);
                }
                Ok(Modifier::default())
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de> {
                let mut modifier = Modifier::default();

                while let Some(key) = map.next_key::<&'de str>()? {
                    match key {
                        "factor" => {
                            modifier.factor = Some(map.next_value()?);
                        }
                        "OR" => {
                            modifier.or = Some(map.next_value()?);
                        }
                        "AND" => {
                            modifier.and = Some(map.next_value()?);
                        }
                        "NAND" => {
                            modifier.nand = Some(map.next_value()?);
                        }
                        "NOT" => {
                            modifier.not = Some(map.next_value()?);
                        }
                        _ => {
                            if let Ok(value) = map.next_value::<String>() {
                                modifier.modifiers.insert(key.to_string(), value);
                            } else {
                                if let Ok(pairs) = map.next_value::<HashMap<String, String>>() {
                                    println!("{:?}", pairs);
                                }
                            }
                        }
                    }
                }
                Ok(modifier)
            }
        }

        deserializer.deserialize_any(ModifierVisitor)
    }
}

#[derive(PartialEq, Debug, Default, Clone, JominiDeserialize, Serialize)]
pub struct TechnologyData {
    pub cost: Option<String>,

    pub tier: Option<String>,

    pub category: Vec<String>,

    #[jomini(duplicated)]
    pub weight: Vec<String>,

    #[jomini(default)]
    pub area: ResearchArea,

    #[jomini(default)]
    pub prerequisites: Vec<String>,

    #[jomini(default)]
    pub start_tech: bool,

    //#[jomini(duplicated, default)]
    //pub modifier: Vec<Modifier>,
}

#[derive(Debug, Default, Serialize, Clone, Eq)]
pub struct Technology {
    pub modid: String,
    pub name: String,

    pub id: String,

    pub localisation: HashMap<Languages, Text>,
    pub cost: u64,
    pub tier: Option<String>,
    pub category: Option<String>,
    pub weight: Option<String>,
    pub area: ResearchArea,
    pub prerequisites: Vec<String>,
    pub start_tech: bool,
}

impl Hash for Technology {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl PartialEq for Technology {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

#[derive(Debug, Default, Serialize)]
pub struct TechnologyNode {
    pub id: String,
    pub data: Rc<Technology>,
    pub prerequisites: Vec<Rc<Technology>>
}

impl Hash for TechnologyNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl PartialEq for TechnologyNode {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

#[derive(Debug)]
pub enum StringOrStruct<T> {
    Str(String),
    Object(T),
}

impl<T> Serialize for StringOrStruct<T> where T: Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self {
            StringOrStruct::Str(s) => serializer.serialize_str(s),
            StringOrStruct::Object(x) => x.serialize(serializer)
        }
    }
}

impl<T> PartialEq for StringOrStruct<T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        if let Self::Str(s1) = &self {
            if let Self::Str(s2) = &other {
                return s1.eq(s2)
            }
        }

        if let Self::Object(s1) = &self {
            if let Self::Object(s2) = &other {
                return s1.eq(s2)
            }
        }

        false
    }
}

impl<T> Clone for StringOrStruct<T> where T: Clone {
    fn clone(&self) -> Self {
        match self {
            StringOrStruct::Str(s) => Self::Str(s.clone()),
            StringOrStruct::Object(x) => Self::Object(x.clone())
        }
    }
}

pub struct StringOrStructVisitor<T>(PhantomData<fn() -> T>);

impl<'de, T> Visitor<'de> for StringOrStructVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = StringOrStruct<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string or map")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(FromStr::from_str(value).unwrap())
    }

    fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
        // into a `Deserializer`, allowing it to be used as the input to T's
        // `Deserialize` implementation. T then deserializes itself using
        // the entries from the map visitor.
        Ok(crate::data::StringOrStruct::Object(
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?,
        ))
    }
}

impl<'de, T> Deserialize<'de> for StringOrStruct<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StringOrStructVisitor(PhantomData))
    }
}

impl<T> FromStr for StringOrStruct<T> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(StringOrStruct::Str(s.to_string()))
    }
}
