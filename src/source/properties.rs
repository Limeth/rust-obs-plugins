use std::io::Write;
use std::fmt::Debug;
use std::path::PathBuf;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use obs_sys::{
    obs_properties_create, obs_properties_destroy,
    obs_data_t, obs_properties_t, obs_property_t,
    obs_data_get_bool, obs_data_get_double, obs_data_get_int, obs_data_get_json, obs_data_get_string,
    obs_data_set_bool, obs_data_set_double, obs_data_set_int, obs_data_set_string,
    obs_data_set_default_bool, obs_data_set_default_double, obs_data_set_default_int, obs_data_set_default_string,
    obs_properties_add_float, obs_properties_add_float_slider, obs_properties_add_int, obs_properties_add_int_slider, obs_properties_add_bool, obs_properties_add_text, obs_properties_add_path, obs_properties_add_color,
    obs_properties_add_button2,
};
use std::sync::Arc;
use std::ffi::{CStr, CString, OsString};
use std::os::raw::{c_char, c_longlong};
use serde_json::Value;

pub mod property_descriptors {
    use super::*;

    pub trait PropertyDescriptorSpecialization: Sized + Clone {
        unsafe fn create_property(
            &self,
            name: *const c_char,
            description: *const c_char,
            properties: *mut obs_properties_t,
        ) -> *mut obs_property_t;
    }

    pub trait ValuePropertyDescriptorSpecialization: PropertyDescriptorSpecialization {
        type ValueType: Debug;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t, default_value: &Self::ValueType) -> Self::ValueType;
        unsafe fn set_property_value(name: *const c_char, data: *mut obs_data_t, value: Self::ValueType);
    }

    #[derive(Clone)]
    pub struct PropertyDescriptorSpecializationBool {}

    impl PropertyDescriptorSpecialization for PropertyDescriptorSpecializationBool {
        unsafe fn create_property(
            &self,
            name: *const c_char,
            description: *const c_char,
            properties: *mut obs_properties_t,
        ) -> *mut obs_property_t {
            obs_properties_add_bool(
                properties,
                name,
                description,
            )
        }
    }

    impl ValuePropertyDescriptorSpecialization for PropertyDescriptorSpecializationBool {
        type ValueType = bool;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t, default_value: &Self::ValueType) -> Self::ValueType {
            obs_data_set_default_bool(data, name, *default_value);
            obs_data_get_bool(data, name)
        }

        unsafe fn set_property_value(name: *const c_char, data: *mut obs_data_t, value: Self::ValueType) {
            obs_data_set_bool(data, name, value);
        }
    }

    #[derive(Clone)]
    pub struct PropertyDescriptorSpecializationI32 {
        pub min: i32,
        pub max: i32,
        pub step: i32,
        pub slider: bool,
    }

    impl PropertyDescriptorSpecialization for PropertyDescriptorSpecializationI32 {
        unsafe fn create_property(
            &self,
            name: *const c_char,
            description: *const c_char,
            properties: *mut obs_properties_t,
        ) -> *mut obs_property_t {
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

    impl ValuePropertyDescriptorSpecialization for PropertyDescriptorSpecializationI32 {
        type ValueType = i32;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t, default_value: &Self::ValueType) -> Self::ValueType {
            obs_data_set_default_int(data, name, *default_value as c_longlong);
            obs_data_get_int(data, name) as i32
        }

        unsafe fn set_property_value(name: *const c_char, data: *mut obs_data_t, value: Self::ValueType) {
            obs_data_set_int(data, name, value as c_longlong);
        }
    }

    #[derive(Clone)]
    pub struct PropertyDescriptorSpecializationF64 {
        pub min: f64,
        pub max: f64,
        pub step: f64,
        pub slider: bool,
    }

    impl PropertyDescriptorSpecialization for PropertyDescriptorSpecializationF64 {
        unsafe fn create_property(
            &self,
            name: *const c_char,
            description: *const c_char,
            properties: *mut obs_properties_t,
        ) -> *mut obs_property_t {
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

    impl ValuePropertyDescriptorSpecialization for PropertyDescriptorSpecializationF64 {
        type ValueType = f64;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t, default_value: &Self::ValueType) -> Self::ValueType {
            obs_data_set_default_double(data, name, *default_value);
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
    pub struct PropertyDescriptorSpecializationString {
        pub string_type: StringType,
    }

    impl PropertyDescriptorSpecialization for PropertyDescriptorSpecializationString {
        unsafe fn create_property(
            &self,
            name: *const c_char,
            description: *const c_char,
            properties: *mut obs_properties_t,
        ) -> *mut obs_property_t {
            obs_properties_add_text(
                properties,
                name,
                description,
                self.string_type as u32,
            )
        }
    }

    impl ValuePropertyDescriptorSpecialization for PropertyDescriptorSpecializationString {
        type ValueType = String;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t, default_value: &Self::ValueType) -> Self::ValueType {
            let c_string = CString::new(default_value.as_str()).expect("Could not convert string to C string.");

            obs_data_set_default_string(data, name, c_string.as_ptr());
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
    pub struct PropertyDescriptorSpecializationPath {
        pub path_type: PathType,
        pub filter: CString,
        pub default_path: CString,
    }

    impl PropertyDescriptorSpecialization for PropertyDescriptorSpecializationPath {
        unsafe fn create_property(
            &self,
            name: *const c_char,
            description: *const c_char,
            properties: *mut obs_properties_t,
        ) -> *mut obs_property_t {
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

    impl ValuePropertyDescriptorSpecialization for PropertyDescriptorSpecializationPath {
        type ValueType = PathBuf;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t, default_value: &Self::ValueType) -> Self::ValueType {
            let c_string_default = CString::new(default_value.to_string_lossy().as_ref())
                .expect("Could not convert string to C string.");

            obs_data_set_default_string(data, name, c_string_default.as_ptr());

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

    pub unsafe extern "C" fn button_callback_global(
        _props: *mut obs_properties_t,
        _property: *mut obs_property_t,
        data: *mut ::std::os::raw::c_void,
    ) -> bool {
        let callback_ptr = data as *mut Arc<Box<dyn Fn() -> bool>>;
        let callback: Box<Arc<Box<dyn Fn() -> bool>>> = Box::from_raw(callback_ptr);
        let result = (callback)();

        std::mem::forget(callback);

        result
    }

    #[derive(Clone)]
    pub struct PropertyDescriptorSpecializationButton {
        callback: Arc<Box<dyn Fn() -> bool>>,
    }

    impl PropertyDescriptorSpecializationButton {
        /// Callback for when a button property is clicked. If the properties
        /// need to be refreshed due to changes to the property layout, return true,
        /// otherwise return false.
        pub fn new(
            callback: Box<dyn Fn() -> bool>,
        ) -> Self {
            Self {
                callback: Arc::new(callback),
            }
        }
    }

    impl PropertyDescriptorSpecialization for PropertyDescriptorSpecializationButton {
        unsafe fn create_property(
            &self,
            name: *const c_char,
            description: *const c_char,
            properties: *mut obs_properties_t,
        ) -> *mut obs_property_t {
            // FIXME: This probably leaks. I am not sure how OBS frees the custom data.
            // Outer box: To be freed by OBS
            // Arc: To enable cloning of the closure
            // Inner box: To enable calling of the closure
            let callback_ptr: *mut Arc<Box<dyn Fn() -> bool>> = Box::into_raw(Box::new(self.callback.clone()));

            obs_properties_add_button2(
                properties,
                name,
                description,
                Some(button_callback_global),
                callback_ptr as *mut _,
            )
        }
    }

    #[derive(Clone)]
    pub struct PropertyDescriptorSpecializationColor;

    #[derive(Clone, Debug, PartialEq)]
    pub struct Color(pub [f32; 4]);

    impl Color {
        pub fn reversed(&self) -> Color {
            Color([self.0[3], self.0[2], self.0[1], self.0[0]])
        }
    }

    impl FromStr for Color {
        type Err = std::num::ParseIntError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            u32::from_str_radix(s, 16).map(|rgba| Color::from(rgba).reversed())
        }
    }

    impl From<[f32; 4]> for Color {
        fn from(inner: [f32; 4]) -> Self {
            Color(inner)
        }
    }

    impl From<u32> for Color {
        /// As defined in vec4.h:
        /// static inline void vec4_from_rgba(struct vec4 *dst, uint32_t rgba)
        fn from(mut rgba: u32) -> Self {
            let x = ((rgba & u8::MAX as u32) as f32 / u8::MAX as f32) as f32;
            rgba >>= 8;
            let y = ((rgba & u8::MAX as u32) as f32 / u8::MAX as f32) as f32;
            rgba >>= 8;
            let z = ((rgba & u8::MAX as u32) as f32 / u8::MAX as f32) as f32;
            rgba >>= 8;
            let w = ((rgba & u8::MAX as u32) as f32 / u8::MAX as f32) as f32;
            [x, y, z, w].into()
        }
    }

    impl From<Color> for [f32; 4] {
        fn from(src: Color) -> Self {
            src.0
        }
    }

    impl From<Color> for u32 {
        /// As defined in vec4.h:
        /// static inline uint32_t vec4_to_rgba(const struct vec4 *src)
        fn from(src: Color) -> Self {
            let mut val = 0;
            val |= ((src[0] as f32 * u8::MAX as f32) as u32) << 0;
            val |= ((src[1] as f32 * u8::MAX as f32) as u32) << 8;
            val |= ((src[2] as f32 * u8::MAX as f32) as u32) << 16;
            val |= ((src[3] as f32 * u8::MAX as f32) as u32) << 24;
            val
        }
    }

    impl Deref for Color {
        type Target = [f32; 4];

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for Color {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl PropertyDescriptorSpecialization for PropertyDescriptorSpecializationColor {
        unsafe fn create_property(
            &self,
            name: *const c_char,
            description: *const c_char,
            properties: *mut obs_properties_t,
        ) -> *mut obs_property_t {
            obs_properties_add_color(
                properties,
                name,
                description,
            )
        }
    }

    impl ValuePropertyDescriptorSpecialization for PropertyDescriptorSpecializationColor {
        type ValueType = Color;

        unsafe fn get_property_value(name: *const c_char, data: *mut obs_data_t, default_value: &Self::ValueType) -> Self::ValueType {
            obs_data_set_default_int(data, name, u32::from(default_value.clone()) as i64);
            Color::from(obs_data_get_int(data, name) as u32)
        }

        unsafe fn set_property_value(name: *const c_char, data: *mut obs_data_t, value: Self::ValueType) {
            obs_data_set_int(data, name, u32::from(value) as i64)
        }
    }

    // TODO: Implement the property kinds below
    #[derive(Clone)]
    pub struct PropertyDescriptorSpecializationList {
        // TODO
    }
    #[derive(Clone)]
    pub struct PropertyDescriptorSpecializationFont {}
    #[derive(Clone)]
    pub struct PropertyDescriptorSpecializationListEditable {
        // TODO
    }
    #[derive(Clone)]
    pub struct PropertyDescriptorSpecializationFrameRate {}
    pub struct PropertyDescriptorSpecializationGroup {
        // Make sure not to `drop` the Properties
        pub properties: Properties,
    }
}

pub use property_descriptors::*;

#[derive(Clone)]
pub struct PropertyDescriptor<T: PropertyDescriptorSpecialization> {
    pub name: CString,
    pub description: CString,
    pub specialization: T,
}

pub struct Properties {
    inner: *mut obs_properties_t,
}

impl Properties {
    pub(crate) unsafe fn from_raw(
        pointer: *mut obs_properties_t,
    ) -> Self {
        Self {
            inner: pointer,
        }
    }

    pub fn new() -> Self {
        unsafe {
            Self::from_raw(obs_properties_create())
        }
    }

    pub(crate) unsafe fn as_raw(&self) -> *mut obs_properties_t {
        self.inner
    }

    pub fn add_property<T: PropertyDescriptorSpecialization>(&mut self, descriptor: &PropertyDescriptor<T>) {
        unsafe {
            descriptor.specialization.create_property(
                descriptor.name.as_ptr(),
                descriptor.description.as_ptr(),
                self.inner,
            );
        }
    }
}

impl Drop for Properties {
    fn drop(&mut self) {
        unsafe {
            obs_properties_destroy(self.inner);
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

    pub fn get_property_value<T: ValuePropertyDescriptorSpecialization>(&mut self, descriptor: &PropertyDescriptor<T>, default_value: &T::ValueType) -> T::ValueType {
        unsafe {
            <T as ValuePropertyDescriptorSpecialization>::get_property_value(descriptor.name.as_ptr(), self.settings, default_value)
        }
    }

    pub fn set_property_value<T: ValuePropertyDescriptorSpecialization>(&mut self, descriptor: &PropertyDescriptor<T>, value: T::ValueType) {
        unsafe {
            <T as ValuePropertyDescriptorSpecialization>::set_property_value(descriptor.name.as_ptr(), self.settings, value);
        }
    }
}
