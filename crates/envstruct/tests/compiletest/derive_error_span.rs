use envstruct::prelude::*;

pub struct Foo {}

#[derive(EnvStruct)]
pub struct Config {
    foo: Foo,
}

fn main() {}
