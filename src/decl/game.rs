use serde::{Deserialize, Serialize};

pub type Game = super::Resource<GameDecl>;
impl super::ResourceType for GameDecl {
    const DIRECTORY: &str = "games";
}

// TODO
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameDecl {
    pub name: String,
}
