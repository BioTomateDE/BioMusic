use serde::{Deserialize, Serialize};

use crate::list::List;

pub type Character = super::Resource<CharacterDecl>;
impl super::ResourceType for CharacterDecl {
    const DIRECTORY: &str = "chars";
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterDecl {
    pub name: String,

    #[serde(skip_serializing_if = "List::is_empty")]
    pub aka: List<String>,
}
