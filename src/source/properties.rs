use std::path::PathBuf;
use obs_sys::{
    obs_data_t, obs_properties_t, obs_property_t,
    obs_data_get_bool, obs_data_get_double, obs_data_get_int, obs_data_get_json, obs_data_get_string,
    obs_data_set_bool, obs_data_set_double, obs_data_set_int, obs_data_set_string,
    obs_properties_add_float, obs_properties_add_float_slider, obs_properties_add_int, obs_properties_add_int_slider, obs_properties_add_bool, obs_properties_add_text, obs_properties_add_path,
};
use std::ffi::{CStr, CString, OsString};
use std::os::raw::{c_char, c_longlong};
use serde_json::Value;

pub mod property_descriptors {
    use super::*;

    pub trait PropertyDescriptor: Sized {
        unsafe fn create_property(&self, name: *const c_char, description: *const c_char, properties: *mut obs_properties_t) -> *mut obs_property_t;
    }

    pub trait ValuePropertyDescriptor: PropertyDescriptor {
        type ValueType;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t) -> Self::ValueType;
        unsafe fn set_property_value(name: *const c_char, data: *mut obs_data_t, value: Self::ValueType);
    }

    #[derive(Clone)]
    pub struct PropertyDescriptorBool {}

    impl PropertyDescriptor for PropertyDescriptorBool {
        unsafe fn create_property(&self, name: *const c_char, description: *const c_char, properties: *mut obs_properties_t) -> *mut obs_property_t {
            obs_properties_add_bool(
                properties,
                name,
                description,
            )
        }
    }

    impl ValuePropertyDescriptor for PropertyDescriptorBool {
        type ValueType = bool;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t) -> Self::ValueType {
            obs_data_get_bool(data, name)
        }

        unsafe fn set_property_value(name: *const c_char, data: *mut obs_data_t, value: Self::ValueType) {
            obs_data_set_bool(data, name, value);
        }
    }

    #[derive(Clone)]
    pub struct PropertyDescriptorI32 {
        pub min: i32,
        pub max: i32,
        pub step: i32,
        pub slider: bool,
    }

    impl PropertyDescriptor for PropertyDescriptorI32 {
        unsafe fn create_property(&self, name: *const c_char, description: *const c_char, properties: *mut obs_properties_t) -> *mut obs_property_t {
            if self.slider {
                obs_properties_add_int_slider(
                    properties,
                    name,
                    description,
                    self.min,
                    self.max,
                    self.step,
                )
            } else {
                obs_properties_add_int(
                    properties,
                    name,
                    description,
                    self.min,
                    self.max,
                    self.step,
                )
            }
        }
    }

    impl ValuePropertyDescriptor for PropertyDescriptorI32 {
        type ValueType = i32;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t) -> Self::ValueType {
            obs_data_get_int(data, name) as i32
        }

        unsafe fn set_property_value(name: *const c_char, data: *mut obs_data_t, value: Self::ValueType) {
            obs_data_set_int(data, name, value as c_longlong);
        }
    }

    #[derive(Clone)]
    pub struct PropertyDescriptorF64 {
        pub min: f64,
        pub max: f64,
        pub step: f64,
        pub slider: bool,
    }

    impl PropertyDescriptor for PropertyDescriptorF64 {
        unsafe fn create_property(&self, name: *const c_char, description: *const c_char, properties: *mut obs_properties_t) -> *mut obs_property_t {
            if self.slider {
                obs_properties_add_float_slider(
                    properties,
                    name,
                    description,
                    self.min,
                    self.max,
                    self.step,
                )
            } else {
                obs_properties_add_float(
                    properties,
                    name,
                    description,
                    self.min,
                    self.max,
                    self.step,
                )
            }
        }
    }

    impl ValuePropertyDescriptor for PropertyDescriptorF64 {
        type ValueType = f64;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t) -> Self::ValueType {
            obs_data_get_double(data, name)
        }

        unsafe fn set_property_value(name: *const c_char, data: *mut obs_data_t, value: Self::ValueType) {
            obs_data_set_double(data, name, value);
        }
    }

    #[repr(u32)]
    #[derive(Clone, Copy)]
    pub enum StringType {
        Default,
        Password,
        Multiline,
    }

    #[derive(Clone)]
    pub struct PropertyDescriptorString {
        pub string_type: StringType,
    }

    impl PropertyDescriptor for PropertyDescriptorString {
        unsafe fn create_property(&self, name: *const c_char, description: *const c_char, properties: *mut obs_properties_t) -> *mut obs_property_t {
            obs_properties_add_text(
                properties,
                name,
                description,
                self.string_type as u32,
            )
        }
    }

    impl ValuePropertyDescriptor for PropertyDescriptorString {
        type ValueType = String;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t) -> Self::ValueType {
            CStr::from_ptr(obs_data_get_string(data, name)).to_string_lossy().to_string()
        }

        unsafe fn set_property_value(name: *const c_char, data: *mut obs_data_t, value: Self::ValueType) {
            let c_string = CString::new(value).expect("Could not convert string to C string.");
            obs_data_set_string(data, name, c_string.as_ptr());
        }
    }

    #[repr(u32)]
    #[derive(Clone, Copy)]
    pub enum PathType {
        File,
        FileSave,
        Directory,
    }

    #[derive(Clone)]
    pub struct PropertyDescriptorPath {
        pub path_type: PathType,
        pub filter: CString,
        pub default_path: CString,
    }

    impl PropertyDescriptor for PropertyDescriptorPath {
        unsafe fn create_property(&self, name: *const c_char, description: *const c_char, properties: *mut obs_properties_t) -> *mut obs_property_t {
            obs_properties_add_path(
                properties,
                name,
                description,
                self.path_type as u32,
                self.filter.as_ptr(),
                self.default_path.as_ptr(),
            )
        }
    }

    impl ValuePropertyDescriptor for PropertyDescriptorPath {
        type ValueType = PathBuf;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t) -> Self::ValueType {
            let c_slice = CStr::from_ptr(obs_data_get_string(data, name)).to_string_lossy();
            let os_string = OsString::from(c_slice.to_string());

            PathBuf::from(os_string)
        }

        unsafe fn set_property_value(name: *const c_char, data: *mut obs_data_t, value: Self::ValueType) {
            let c_string = CString::new(value.to_string_lossy().as_ref())
                .expect("Could not convert string to C string.");
            obs_data_set_string(data, name, c_string.as_ptr());
        }
    }

    // TODO: Implement the property kinds below
    #[derive(Clone)]
    pub struct PropertyDescriptorList {
        // TODO
    }
    pub struct PropertyDescriptorButton {
        /// Callback for when a button property is clicked. If the properties
        /// need to be refreshed due to changes to the property layout, return true,
        /// otherwise return false.
        pub callback: Box<dyn Fn() -> bool>,
    }
    #[derive(Clone)]
    pub struct PropertyDescriptorFont {}
    #[derive(Clone)]
    pub struct PropertyDescriptorListEditable {
        // TODO
    }
    #[derive(Clone)]
    pub struct PropertyDescriptorFrameRate {}
    pub struct PropertyDescriptorGroup {
        pub properties: Properties,
    }
}

pub use property_descriptors::*;

pub struct Property<T: PropertyDescriptor> {
    inner: *mut obs_property_t,
    name: CString,
    description: CString,
    descriptor: T,
}

// pub(crate) struct Property {
//     name: &'static str,
//     property_type: PropertyType,
// }

// enum PropertyType {
//     Float(f64, f64),
//     Int(i32, i32),
// }

pub struct Properties {
    inner: *mut obs_properties_t,
}

// pub struct Properties<'a> {
//     pointer: *mut obs_properties_t,
//     properties: &'a mut Vec<Property>,
// }

impl Properties {
    pub(crate) unsafe fn from_raw(
        pointer: *mut obs_properties_t,
    ) -> Self {
        Self {
            inner: pointer,
        }
    }

    /// # Safety
    /// Modifying this pointer could cause UB
    pub unsafe fn into_raw(self) -> *mut obs_properties_t {
        self.inner
    }

    pub fn add_property<T: PropertyDescriptor>(&mut self, name: String, description: String, property_descriptor: T) -> Property<T> {
        let name = CString::new(name).expect("Invalid property name.");
        let description = CString::new(description).expect("Invalid property description.");
        let inner = unsafe {
            property_descriptor.create_property(name.as_ptr(), description.as_ptr(), self.inner)
        };

        Property {
            inner,
            name,
            description,
            descriptor: property_descriptor,
        }
    }
}

pub struct SettingsContext {
    settings: *mut obs_data_t,
    init_data: Option<Value>,
}

impl SettingsContext {
    pub(crate) unsafe fn from_raw(settings: *mut obs_data_t) -> Self {
        SettingsContext {
            settings,
            init_data: None,
        }
    }

    pub(crate) unsafe fn as_raw(&self) -> *mut obs_data_t {
        self.settings
    }

    fn get_data(&mut self) -> &Option<Value> {
        let mut json_data: Option<Value> = None;

        if self.init_data.is_none() {
            let data = unsafe { CStr::from_ptr(obs_data_get_json(self.settings)) };
            if let Some(value) = data
                .to_str()
                .ok()
                .and_then(|x| serde_json::from_str(x).ok())
            {
                json_data = Some(value);
            }
        }

        if let Some(data) = json_data {
            self.init_data.replace(data);
        }

        &self.init_data
    }

    pub fn get_property_value<T: ValuePropertyDescriptor>(&self, property: &Property<T>) -> T::ValueType {
        unsafe {
            <T as ValuePropertyDescriptor>::get_property_value(property.name.as_ptr(), self.settings)
        }
    }

    pub fn set_property_value<T: ValuePropertyDescriptor>(&self, property: &Property<T>, value: T::ValueType) {
        unsafe {
            <T as ValuePropertyDescriptor>::set_property_value(property.name.as_ptr(), self.settings, value);
        }
    }
}
