use std::borrow::Cow;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Visitor};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ActiveProgram {
    pub program: String,
    pub subprogram: Option<String>,
}

impl Serialize for ActiveProgram {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer {
            if let Some(subprogram) = &self.subprogram {
                serializer.serialize_str(&format!("{}|{}", self.program, subprogram.as_str()))
            } else {
                serializer.serialize_str(&self.program)
            }
    }
}

impl<'de> Deserialize<'de> for ActiveProgram {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de> {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = ActiveProgram;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a string with at most one bar symbol |")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                    E: serde::de::Error, {
                let items: Vec<_> = v.split("|").collect();
                match items.len() {
                    1 => {
                        Ok(ActiveProgram {
                            program: items[0].to_owned(),
                            subprogram: None,
                        })
                    },
                    2 => {
                        Ok(ActiveProgram {
                            program: items[0].to_owned(),
                            subprogram: Some(items[1].to_owned())
                        })
                    },
                    _ => {
                        dbg!("!!");
                        Err(E::custom("couldn't parse ActiveProgram: string contains more than one bar symbol |"))
                    }
                }
            }
        }

        deserializer.deserialize_string(Visitor)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct Program {
    pub program: String,
}

fn predict_subprogram<'a, 'b>(program: &'a str, title: &'b str) -> Cow<'b, str> {
    let mut browser_title = None;

    let subprogram = match program.to_lowercase().as_str() {
        "firefox" => {
            if title.ends_with("— Mozilla Firefox") {
                browser_title = Some(title[0..title.len()-"— Mozilla Firefox".len()].trim());
            }
            "".into()
        },
        "code" => {
            let slices: Vec<_> = title.split(" - ").collect();
            if slices.len() == 3 {
                slices[1]
            } else {
                "".into()
            }
        },
        _ => "".into()
    };

    if let Some(title) = browser_title {
        if title == "generals.io" || title.starts_with("generals.io |") {
            return "generals.io".into()
        }
        if title.ends_with("YouTube") {
            return "youtube.com".into()
        }
        if title.ends_with("| Musescore.com") || title.starts_with("Musescore.com |") {
            return "musescore.com".into()
        }
        if title.ends_with("- Google Docs") {
            return "docs.google.com".into()
        }
        if title == "WhatsApp" {
            return "whatsapp.com".into();
        }
        if title.ends_with("| Quizlet") {
            return "quizlet.com".into();
        }
    }

    subprogram.into()
}

fn normalize(program: &str) -> Cow<str> {
    let program = program.replace("-", " ");

    program.split(" ").map(|slice| {
        let slice = slice.split(".").collect::<Vec<&str>>();
        let slice = slice[slice.len()-1];
        let mut chars = slice.chars();
        let first = match chars.next() {
            Some(c) => c,
            None => return "".to_owned()
        };

        format!("{}{}", first.to_uppercase(), chars.collect::<String>().to_lowercase())
    }).collect::<Vec<String>>().join(" ").into()
}

pub trait RawWindowData {
    fn program(&self) -> Cow<'_, str>;
    fn title(&self) -> Cow<'_, str>;
}

impl<T> From<T> for ActiveProgram where T: RawWindowData {
    fn from(item: T) -> Self {
        let (program, title) = (item.program(), item.title());
        let sub = predict_subprogram(&program, &title);
        ActiveProgram {
            program: normalize(&program).into_owned(),
            subprogram: if sub.is_empty() { None } else { Some(sub.into_owned()) }
        }
    }
}

impl<T> From<T> for Program where T: RawWindowData {
    fn from(item: T) -> Self {
        Program {
            program: normalize(&item.program()).into_owned()
        }
    }
}


pub mod http;