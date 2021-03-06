//! # Rust OBS Wrapper
//!
//! A safe wrapper around the OBS API, useful for creating OBS sources, filters and effects.
//!
//! ## Usage
//!
//! In your `Cargo.toml` file add the following section, substituting `<module-name>` for the name of
//! the module:
//!
//! ```toml
//! [dependencies]
//! obs-wrapper = "0.1"
//!
//! [lib]
//! name = "<module-name>"
//! crate-type = ["cdylib"]
//! ```
//!
//! The process for creating a plugin is:
//! 1. Create a struct that implements Module
//! 1. Create a struct that will store the plugin state
//! 1. Implement the required traits for the module
//! 1. Enable the traits which have been enabled in the module `load` method
//!
//! ~~~
//! use obs_wrapper::{
//!     // Everything required for modules
//!     prelude::*,
//!     // Everything required for creating a source
//!     source::*,
//!     // Macro for registering modules
//!     obs_register_module,
//!     // Macro for creating strings
//!     cstr,
//! };
//!
//! // The module that will handle creating the source.
//! struct TestModule {
//!     context: ModuleContext
//! };
//!
//! // The source that will be shown inside OBS.
//! struct TestSource;
//!
//! // The state of the source that is managed by OBS and used in each trait method.
//! struct SourceData;
//!
//! // Implement the Sourceable trait for TestSource, this is required for each source.
//! // It allows you to specify the source ID and type.
//! impl Sourceable for TestSource {
//!     fn get_id() -> &'static CStr {
//!         cstr!("test_source")
//!     }
//!
//!     fn get_type() -> SourceType {
//!         SourceType::FILTER
//!     }
//! }
//!
//! // Allow OBS to show a name for the source
//! impl GetNameSource<SourceData> for TestSource {
//!     fn get_name() -> &'static CStr {
//!         cstr!("Test Source")
//!     }
//! }
//!
//! // Implement the Module trait for TestModule. This will handle the creation of the source and
//! // has some methods for telling OBS a bit about itself.
//! impl Module for TestModule {
//!     fn new(context: ModuleContext) -> Self {
//!         Self { context }
//!     }
//!
//!     fn get_ctx(&self) -> &ModuleContext {
//!         &self.context
//!     }
//!    
//!     // Load the module - create all sources, returning true if all went well.
//!     fn load(&mut self, load_context: &mut LoadContext) -> bool {
//!         // Create the source
//!         let source = load_context
//!             .create_source_builder::<TestSource, SourceData>()
//!             // Since GetNameSource is implemented, this method needs to be called to
//!             // enable it.
//!             .enable_get_name()
//!             .build();
//!    
//!         // Tell OBS about the source so that it will show it.
//!         load_context.register_source(source);
//!    
//!         // Nothing could have gone wrong, so return true.
//!         true
//!     }
//!    
//!     fn description() -> &'static CStr {
//!         cstr!("A great test module.")
//!     }
//!
//!     fn name() -> &'static CStr {
//!         cstr!("Test Module")
//!     }
//!
//!     fn author() -> &'static CStr {
//!         cstr!("Bennett")
//!     }
//! }
//! ~~~
//!
//! ### Installing
//!
//! 1. Run `cargo build --release`
//! 2. Copy `/target/release/<module-name>.so` to your OBS plugins folder (`/usr/lib/obs-plugins/`)
//! 3. The plugin should be available for use from inside OBS

#![feature(never_type)]
#![feature(arbitrary_self_types)]

/// Raw bindings of OBS C API
pub use obs_sys;
pub use cstr::*;

/// Utilities
pub mod util;
/// Context abstractions
pub mod context;
/// Globally accessible info
pub mod info;
/// Tools required for manipulating graphics in OBS
pub mod graphics;
/// Methods for logging to OBS console
pub mod log;
/// Tools for creating modules
pub mod module;
/// Tools for creating sources
pub mod source;
/// Tools for handling audio
pub mod audio;

/// Re-exports of a bunch of popular tools
pub mod prelude {
    pub use crate::module::*;
    pub use cstr::*;
}
