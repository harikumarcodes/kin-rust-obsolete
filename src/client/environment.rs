use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

/// An Agora Environment.
#[derive(FromPrimitive)]
pub enum Environment {
    /// Kin production blockchain.
    Production = 1,

    /// Kin test blockchain.
    Test = 2,
}

impl Environment {
    /// Converts u32 to Environment. Defaults to Environment::Test.
    pub fn from_u32(value: u32) -> Environment {
        if let Some(environment) = FromPrimitive::from_u32(value) {
            environment
        } else {
            Environment::Test
        }
    }
}
