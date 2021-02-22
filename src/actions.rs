use serde::Deserialize;
use std::fs;

#[derive(Clone, Debug, Deserialize)]
pub struct Actions {
    pub action: Vec<Action>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Action {
    pub name: String,
    pub commands: Vec<String>,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub fn gen_actions() -> Actions {
    let actions_toml =
        &fs::read_to_string("config/actions.toml").expect("unable to open actions.toml");
    toml::from_str::<Actions>(actions_toml).unwrap()
}
