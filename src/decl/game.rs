use serde::Deserialize;
use serde::Serialize;

pub type Game = super::Resource<GameDecl>;
impl super::ResourceType for GameDecl {
    const DIRECTORY: &str = "games";
    const NAME: &str = "game declaration";
}

// TODO
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameDecl {
    pub name: String,
}
