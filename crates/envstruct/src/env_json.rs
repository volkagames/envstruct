#![cfg(feature = "serde_json")]

use crate::*;

pub struct EnvJson<T: for<'a> serde::de::Deserialize<'a>>(T);

impl<T: for<'a> serde::de::Deserialize<'a>> EnvParsePrimitive for EnvJson<T> {
    fn parse(val: &str) -> Result<Self, StdError> {
        Ok(EnvJson(serde_json::from_str(val)?))
    }
}

impl<T: for<'a> serde::de::Deserialize<'a>> AsRef<T> for EnvJson<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T: for<'a> serde::de::Deserialize<'a>> std::ops::Deref for EnvJson<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: for<'a> serde::de::Deserialize<'a>> std::ops::DerefMut for EnvJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
