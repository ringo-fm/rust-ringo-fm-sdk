//! `GenerationSchema` builder over `FMGenerationSchema*` C calls.

use std::ffi::{c_void, CString};

use ringo_fm_sys as sys;

use crate::error::{Error, Result};
use crate::handle::{check_error, FmString, ManagedRef};

pub(crate) struct SchemaTag;
pub(crate) struct SchemaPropertyTag;

/// Constraint applied to a [`GenerationSchemaProperty`].
#[derive(Debug, Clone)]
pub enum GenerationGuide {
    /// One of a fixed set of values.
    AnyOf { choices: Vec<String>, wrapped: bool },
    /// Exact element count (array properties).
    Count { count: i32, wrapped: bool },
    Maximum { value: f64, wrapped: bool },
    Minimum { value: f64, wrapped: bool },
    Range { min: f64, max: f64, wrapped: bool },
    Regex { pattern: String, wrapped: bool },
    MinItems(i32),
    MaxItems(i32),
}

/// One property of a [`GenerationSchema`].
pub struct GenerationSchemaProperty {
    pub(crate) handle: ManagedRef<SchemaPropertyTag>,
}

impl GenerationSchemaProperty {
    pub fn new(
        name: &str,
        description: Option<&str>,
        type_name: &str,
        is_optional: bool,
    ) -> Result<Self> {
        let name_c = CString::new(name).map_err(|e| Error::Native(e.to_string()))?;
        let type_c = CString::new(type_name).map_err(|e| Error::Native(e.to_string()))?;
        let desc_c = match description {
            Some(d) => Some(CString::new(d).map_err(|e| Error::Native(e.to_string()))?),
            None => None,
        };
        let desc_ptr = desc_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
        let ptr = unsafe {
            sys::FMGenerationSchemaPropertyCreate(name_c.as_ptr(), desc_ptr, type_c.as_ptr(), is_optional)
        };
        Ok(Self { handle: ManagedRef::from_owned(ptr)? })
    }

    pub fn add_guide(self, guide: &GenerationGuide) -> Result<Self> {
        let p = self.handle.as_ptr();
        match guide {
            GenerationGuide::AnyOf { choices, wrapped } => {
                let cstrs: Vec<CString> = choices
                    .iter()
                    .map(|s| CString::new(s.as_str()).map_err(|e| Error::Native(e.to_string())))
                    .collect::<Result<_>>()?;
                let ptrs: Vec<*const i8> = cstrs.iter().map(|c| c.as_ptr()).collect();
                unsafe {
                    sys::FMGenerationSchemaPropertyAddAnyOfGuide(
                        p,
                        ptrs.as_ptr() as *mut _,
                        ptrs.len() as i32,
                        *wrapped,
                    )
                };
            }
            GenerationGuide::Count { count, wrapped } => unsafe {
                sys::FMGenerationSchemaPropertyAddCountGuide(p, *count, *wrapped);
            },
            GenerationGuide::Maximum { value, wrapped } => unsafe {
                sys::FMGenerationSchemaPropertyAddMaximumGuide(p, *value, *wrapped);
            },
            GenerationGuide::Minimum { value, wrapped } => unsafe {
                sys::FMGenerationSchemaPropertyAddMinimumGuide(p, *value, *wrapped);
            },
            GenerationGuide::Range { min, max, wrapped } => unsafe {
                sys::FMGenerationSchemaPropertyAddRangeGuide(p, *min, *max, *wrapped);
            },
            GenerationGuide::Regex { pattern, wrapped } => {
                let pat_c = CString::new(pattern.as_str()).map_err(|e| Error::Native(e.to_string()))?;
                unsafe { sys::FMGenerationSchemaPropertyAddRegex(p, pat_c.as_ptr(), *wrapped) };
            }
            GenerationGuide::MinItems(n) => unsafe {
                sys::FMGenerationSchemaPropertyAddMinItemsGuide(p, *n);
            },
            GenerationGuide::MaxItems(n) => unsafe {
                sys::FMGenerationSchemaPropertyAddMaxItemsGuide(p, *n);
            },
        }
        Ok(self)
    }
}

/// Structured-generation schema.
pub struct GenerationSchema {
    pub(crate) handle: ManagedRef<SchemaTag>,
}

impl GenerationSchema {
    pub fn new(name: &str, description: Option<&str>) -> Result<Self> {
        let name_c = CString::new(name).map_err(|e| Error::Native(e.to_string()))?;
        let desc_c = match description {
            Some(d) => Some(CString::new(d).map_err(|e| Error::Native(e.to_string()))?),
            None => None,
        };
        let desc_ptr = desc_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
        let ptr = unsafe { sys::FMGenerationSchemaCreate(name_c.as_ptr(), desc_ptr) };
        Ok(Self { handle: ManagedRef::from_owned(ptr)? })
    }

    pub fn add_property(&mut self, prop: GenerationSchemaProperty) -> &mut Self {
        unsafe { sys::FMGenerationSchemaAddProperty(self.handle.as_ptr(), prop.handle.as_ptr()) };
        self
    }

    pub fn add_reference_schema(&mut self, other: &GenerationSchema) -> &mut Self {
        unsafe {
            sys::FMGenerationSchemaAddReferenceSchema(self.handle.as_ptr(), other.handle.as_ptr())
        };
        self
    }

    pub fn to_json(&self) -> Result<String> {
        let mut code: i32 = 0;
        let mut desc: *mut i8 = std::ptr::null_mut();
        let ptr = unsafe { sys::FMGenerationSchemaGetJSONString(self.handle.as_ptr(), &mut code, &mut desc) };
        check_error(code, desc)?;
        FmString::from_raw(ptr)
            .ok_or_else(|| Error::Native("schema JSON null".into()))?
            .to_string()
    }

    pub(crate) fn as_ptr(&self) -> *const c_void {
        self.handle.as_ptr()
    }
}
