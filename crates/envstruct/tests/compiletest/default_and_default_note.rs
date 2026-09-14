use envstruct::prelude::*;

#[derive(EnvStruct)]
pub struct Config {
    #[env(default = "4", default_note = "physical CPU count")]
    pub workers: Option<usize>,
}

fn main() {}
