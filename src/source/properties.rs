use super::ObsString;
use obs_sys::{
    obs_data_t, obs_properties_t, obs_property_t,
    obs_data_get_bool, obs_data_get_double, obs_data_get_int, obs_data_get_json,
    obs_properties_add_float, obs_properties_add_float_slider, obs_properties_add_int, obs_properties_add_int_slider, obs_properties_add_bool,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use serde_json::Value;

pub mod property_descriptors {
    use super::*;

    pub trait PropertyDescriptor: Sized {
        unsafe fn create_property(&self, name: *const c_char, description: *const c_char, properties: *mut obs_properties_t) -> *mut obs_property_t;
    }

    pub trait ValuePropertyDescriptor: PropertyDescriptor {
        type ValueType;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t) -> Self::ValueType;
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
    }

    // TODO: Implement the property kinds below
    #[derive(Clone)]
    pub struct PropertyDescriptorString {}
    #[derive(Clone)]
    pub struct PropertyDescriptorPath {
        pub filter: String,
        pub default_path: String,
    }
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

    // pub fn add_float_slider(
    //     &mut self,
    //     name: ObsString,
    //     description: ObsString,
    //     min: f64,
    //     max: f64,
    //     step: f64,
    // ) -> &mut Self {
    //     unsafe {
    //         self.properties.push(Property {
    //             name: name.as_str(),
    //             property_type: PropertyType::Float(min, max),
    //         });
    //         obs_properties_add_float_slider(
    //             self.pointer,
    //             name.as_ptr(),
    //             description.as_ptr(),
    //             min,
    //             max,
    //             step,
    //         );
    //     }
    //     self
    // }

    // pub fn add_float(
    //     &mut self,
    //     name: ObsString,
    //     description: ObsString,
    //     min: f64,
    //     max: f64,
    //     step: f64,
    // ) -> &mut Self {
    //     unsafe {
    //         self.properties.push(Property {
    //             name: name.as_str(),
    //             property_type: PropertyType::Float(min, max),
    //         });
    //         obs_properties_add_float(
    //             self.pointer,
    //             name.as_ptr(),
    //             description.as_ptr(),
    //             min,
    //             max,
    //             step,
    //         );
    //     }
    //     self
    // }

    // pub fn add_int(
    //     &mut self,
    //     name: ObsString,
    //     description: ObsString,
    //     min: i32,
    //     max: i32,
    //     step: i32,
    // ) -> &mut Self {
    //     unsafe {
    //         self.properties.push(Property {
    //             name: name.as_str(),
    //             property_type: PropertyType::Int(min, max),
    //         });
    //         obs_properties_add_int(
    //             self.pointer,
    //             name.as_ptr(),
    //             description.as_ptr(),
    //             min,
    //             max,
    //             step,
    //         );
    //     }
    //     self
    // }
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

    // pub fn get_float(&mut self, param: ObsString) -> Option<f64> {
    //     if let Some(Property {
    //         property_type: PropertyType::Float(min, max),
    //         ..
    //     }) = self
    //         .properties
    //         .iter()
    //         .filter(|p| {
    //             matches!(p.property_type, PropertyType::Float(_, _)) && p.name == param.as_str()
    //         })
    //         .next()
    //     {
    //         Some(
    //             (unsafe { obs_data_get_double(self.settings, param.as_ptr()) })
    //                 .min(*max)
    //                 .max(*min),
    //         )
    //     } else {
    //         if let Some(data) = self.get_data() {
    //             let param = param.as_str();
    //             if let Some(val) = data.get(&param[..param.len() - 1]) {
    //                 return val.as_f64();
    //             }
    //         }

    //         None
    //     }
    // }

    // pub fn get_int(&mut self, param: ObsString) -> Option<i32> {
    //     if let Some(Property {
    //         property_type: PropertyType::Int(min, max),
    //         ..
    //     }) = self
    //         .properties
    //         .iter()
    //         .filter(|p| {
    //             matches!(p.property_type, PropertyType::Int(_, _)) && p.name == param.as_str()
    //         })
    //         .next()
    //     {
    //         Some(
    //             (unsafe { obs_data_get_int(self.settings, param.as_ptr()) } as i32)
    //                 .min(*max)
    //                 .max(*min),
    //         )
    //     } else {
    //         if let Some(data) = self.get_data() {
    //             let param = param.as_str();
    //             if let Some(val) = data.get(&param[..param.len() - 1]) {
    //                 if let Some(val) = val.as_i64() {
    //                     return Some(val as i32);
    //                 }
    //             }
    //         }

    //         None
    //     }
    // }
}
