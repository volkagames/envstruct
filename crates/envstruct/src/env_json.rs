#![cfg(feature = "serde_json")]

use crate::*;

/// Wrapper struct for deserializing JSON environment variables
pub struct EnvJson<T: for<'a> serde::de::Deserialize<'a>>(T);

/// Implementation of EnvParsePrimitive for EnvJson
impl<T: for<'a> serde::de::Deserialize<'a>> EnvParsePrimitive for EnvJson<T> {
    /// Parses a JSON string into an EnvJson instance
    ///
    /// # Arguments
    ///
    /// * `val` - A string slice that holds the JSON data
    ///
    /// # Returns
    ///
    /// * `Result<Self, BoxError>` - An instance of EnvJson or an error
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(EnvJson(serde_json::from_str(val)?))
    }
}

/// Implementation of AsRef to get a reference to the inner value
impl<T: for<'a> serde::de::Deserialize<'a>> AsRef<T> for EnvJson<T> {
    /// Returns a reference to the inner value
    ///
    /// # Returns
    ///
    /// * `&T` - A reference to the inner value
    fn as_ref(&self) -> &T {
        &self.0
    }
}

/// Implementation of Deref to allow dereferencing to the inner value
impl<T: for<'a> serde::de::Deserialize<'a>> std::ops::Deref for EnvJson<T> {
    type Target = T;
    /// Dereferences to the inner value
    ///
    /// # Returns
    ///
    /// * `&Self::Target` - A reference to the inner value
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Implementation of DerefMut to allow mutable dereferencing to the inner value
impl<T: for<'a> serde::de::Deserialize<'a>> std::ops::DerefMut for EnvJson<T> {
    /// Mutably dereferences to the inner value
    ///
    /// # Returns
    ///
    /// * `&mut Self::Target` - A mutable reference to the inner value
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
