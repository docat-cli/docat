use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::fmt::Formatter;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Service {
    #[serde(rename(deserialize = "Service"))]
    pub name: String,
    #[serde(rename(deserialize = "State"))]
    pub status: Status,
}

#[derive(Serialize, PartialEq, Debug, Clone)]
pub enum Status {
    Up,
    Down,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Status::Up => write!(f, "up  "),
            Status::Down => write!(f, "down"),
        }
    }
}

impl<'de> Deserialize<'de> for Status {
    fn deserialize<D>(deserializer: D) -> anyhow::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?.to_lowercase();
        let state = match string.as_str() {
            "running" => Status::Up,
            _ => Status::Down,
        };
        Ok(state)
    }
}
