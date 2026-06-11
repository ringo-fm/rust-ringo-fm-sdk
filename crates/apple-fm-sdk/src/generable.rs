//! `Generable` — types the model can produce via structured generation.
//!
//! V1 ships the trait only; an automatic `#[derive(Generable)]` proc macro is a
//! follow-up. Implement manually:
//!
//! ```ignore
//! use apple_fm_sdk::{Generable, GenerationSchema, GenerationSchemaProperty, GeneratedContent, Result};
//!
//! struct Person { name: String, age: i32 }
//!
//! impl Generable for Person {
//!     fn schema() -> Result<GenerationSchema> {
//!         let mut s = GenerationSchema::new("Person", None)?;
//!         s.add_property(GenerationSchemaProperty::new("name", None, "String", false)?);
//!         s.add_property(GenerationSchemaProperty::new("age",  None, "Int",    false)?);
//!         Ok(s)
//!     }
//!     fn from_generated(content: &GeneratedContent) -> Result<Self> {
//!         Ok(Person {
//!             name: content.get_property("name")?,
//!             age: content.get_property("age")?.parse().map_err(|e| apple_fm_sdk::Error::Native(format!("{e}")))?,
//!         })
//!     }
//! }
//! ```

use crate::error::Result;
use crate::generated::GeneratedContent;
use crate::schema::GenerationSchema;

pub trait Generable: Sized {
    fn schema() -> Result<GenerationSchema>;
    fn from_generated(content: &GeneratedContent) -> Result<Self>;
}
