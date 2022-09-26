use std::collections::HashMap;
use std::fmt;
use std::fmt::{Formatter, Write};
use std::marker::PhantomData;
use std::str::FromStr;
use jomini::{DeserializeError, JominiDeserialize, TextDeserializer, TextTape, TextToken, Windows1252Encoding};
use jomini::text::{GroupEntry, ObjectReader};
use serde::{de, Deserialize, Deserializer};
use serde::de::{MapAccess, Visitor};

/// Not very useful currently
enum ResearchArea {
    Society, Physics, Engineering
}

#[derive(PartialEq, Debug, JominiDeserialize)]
pub struct Technology {

    pub cost: Option<String>,

    pub tier: Option<String>,

    #[jomini(default)]
    pub category: Vec<String>,

    #[jomini(default, duplicated)]
    pub weight: Vec<String>,

    pub area: Option<String>,
    pub prerequisites: Option<Vec<String>>
}

#[derive(Debug)]
pub enum StringOrStruct<T> {
    Str(String),
    Object(T)
}

pub struct StringOrStructVisitor<T>(PhantomData<fn() -> T>);

impl<'de, T> Visitor<'de> for StringOrStructVisitor<T> where T: Deserialize<'de> {
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
        Ok(crate::data::StringOrStruct::Object(Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?))
    }
}

impl<'de, T> Deserialize<'de> for StringOrStruct<T> where T: Deserialize<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {

        deserializer.deserialize_any(StringOrStructVisitor(PhantomData))
    }
}

impl<T> FromStr for StringOrStruct<T> {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(StringOrStruct::Str(s.to_string()))
    }
}

#[derive(PartialEq, Debug)]
pub struct Technologies(pub Vec<(String, Technology)>);

pub fn deserialize_technologies(tape: TextTape) -> HashMap<String, Technology> {
    /*for token in tape.tokens() {
        match token {
            TextToken::Object()
            _ => println!("{:?}", token)/*
            TextToken::Array(_) => {}
            TextToken::Object(_) => {}
            TextToken::HiddenObject(_) => {}
            TextToken::Unquoted(_) => {}
            TextToken::Quoted(_) => {}
            TextToken::Parameter(_) => {}
            TextToken::UndefinedParameter(_) => {}
            TextToken::Operator(_) => {}
            TextToken::End(_) => {}
            TextToken::Header(_) => {}*/
        }
    }*//*
    let reader = tape.windows1252_reader();
    for (key, _op, value) in reader.fields() {
        let key_str = key.read_str();
        if key_str.starts_with('@') {
            let value = value.read_str().unwrap();
            println!("Variable {} = {}", key_str, value);
        } else {
            match value.read_object() {
                Ok(obj) => {
                    TextDeserializer::from_windows1252_tape(value.token())
                }
                Err(_) => {}
            }
        }
    }*/

    HashMap::new()
}

impl<'de> Deserialize<'de> for Technologies {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        struct TechnologiesVisitor;

        impl<'de> de::Visitor<'de> for TechnologiesVisitor {
            type Value = Technologies;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("struct Technologies")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de> {
                let mut values = if let Some(size) = map.size_hint() {
                    Vec::with_capacity(size)
                } else {
                    Vec::new()
                };

                while let Some(key) = map.next_key::<&str>()? {
                    let val = match key {
                        _ => {
                            return Err(de::Error::custom(format!("unknown battle key: {}", &key)))
                        }
                    };

                    values.push(val);
                }

                Ok(Technologies(values))
            }
        }

        deserializer.deserialize_map(TechnologiesVisitor)
    }
}
