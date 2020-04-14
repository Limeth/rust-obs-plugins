use std::fmt::Debug;
use std::mem;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use core::convert::TryFrom;
use obs_sys::{
    size_t,
    graphics_t,
    gs_get_context,
    gs_address_mode, gs_address_mode_GS_ADDRESS_BORDER, gs_address_mode_GS_ADDRESS_CLAMP,
    gs_address_mode_GS_ADDRESS_MIRROR, gs_address_mode_GS_ADDRESS_MIRRORONCE,
    gs_address_mode_GS_ADDRESS_WRAP, gs_color_format, gs_color_format_GS_A8,
    gs_color_format_GS_BGRA, gs_color_format_GS_BGRX, gs_color_format_GS_DXT1,
    gs_color_format_GS_DXT3, gs_color_format_GS_DXT5, gs_color_format_GS_R10G10B10A2,
    gs_color_format_GS_R16, gs_color_format_GS_R16F, gs_color_format_GS_R32F,
    gs_color_format_GS_R8, gs_color_format_GS_R8G8, gs_color_format_GS_RG16F,
    gs_color_format_GS_RG32F, gs_color_format_GS_RGBA, gs_color_format_GS_RGBA16,
    gs_color_format_GS_RGBA16F, gs_color_format_GS_RGBA32F, gs_color_format_GS_UNKNOWN,
    gs_effect_create, gs_effect_destroy, gs_effect_get_param_by_name, gs_effect_get_param_info,
    gs_effect_param_info, gs_effect_set_next_sampler, gs_effect_t, gs_eparam_t,
    gs_sample_filter, gs_sample_filter_GS_FILTER_ANISOTROPIC, gs_sample_filter_GS_FILTER_LINEAR,
    gs_sample_filter_GS_FILTER_MIN_LINEAR_MAG_MIP_POINT,
    gs_sample_filter_GS_FILTER_MIN_LINEAR_MAG_POINT_MIP_LINEAR,
    gs_sample_filter_GS_FILTER_MIN_MAG_LINEAR_MIP_POINT,
    gs_sample_filter_GS_FILTER_MIN_MAG_POINT_MIP_LINEAR,
    gs_sample_filter_GS_FILTER_MIN_POINT_MAG_LINEAR_MIP_POINT,
    gs_sample_filter_GS_FILTER_MIN_POINT_MAG_MIP_LINEAR, gs_sample_filter_GS_FILTER_POINT,
    gs_sampler_info, gs_samplerstate_create, gs_samplerstate_destroy, gs_samplerstate_t,
    gs_shader_param_type, gs_shader_param_type_GS_SHADER_PARAM_BOOL,
    gs_shader_param_type_GS_SHADER_PARAM_FLOAT, gs_shader_param_type_GS_SHADER_PARAM_INT,
    gs_shader_param_type_GS_SHADER_PARAM_INT2, gs_shader_param_type_GS_SHADER_PARAM_INT3,
    gs_shader_param_type_GS_SHADER_PARAM_INT4, gs_shader_param_type_GS_SHADER_PARAM_MATRIX4X4,
    gs_shader_param_type_GS_SHADER_PARAM_STRING, gs_shader_param_type_GS_SHADER_PARAM_TEXTURE,
    gs_shader_param_type_GS_SHADER_PARAM_UNKNOWN, gs_shader_param_type_GS_SHADER_PARAM_VEC2,
    gs_shader_param_type_GS_SHADER_PARAM_VEC3, gs_shader_param_type_GS_SHADER_PARAM_VEC4,
    obs_allow_direct_render, obs_allow_direct_render_OBS_ALLOW_DIRECT_RENDERING,
    obs_allow_direct_render_OBS_NO_DIRECT_RENDERING, obs_enter_graphics, obs_leave_graphics, vec2,
    gs_texture_t,
    gs_effect_get_num_params,
    gs_effect_get_param_by_idx,
    gs_effect_get_default_val_size,
    gs_effect_get_default_val,
    vec3, vec4,
    gs_effect_set_bool,
    gs_effect_set_float,
    gs_effect_set_int,
    gs_effect_set_vec2,
    gs_effect_set_vec3,
    gs_effect_set_vec4,
    gs_effect_set_val,
    gs_effect_set_texture,
    gs_effect_set_matrix4,
};
use paste::item;
use cstr::cstr;
use crate::context::*;

mod texture;

pub use texture::*;

pub mod shader_param_types {
    use super::*;

    pub trait ShaderParamType {
        type RustType: Debug;

        /// May only be called in a graphics context.
        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType);

        /// May only be called in a graphics context.
        unsafe fn get_param_value_default<'a>(param: *mut gs_eparam_t) -> &'a Self::RustType {
            // This test does not seem to be passing, but the values seem to be right.
            // assert_eq!(gs_effect_get_default_val_size(param) as usize, std::mem::size_of::<Self::RustType>());
            let ptr = gs_effect_get_default_val(param);

            &*(ptr as *const Self::RustType)
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind;
    }

    pub struct ShaderParamTypeBool;
    impl ShaderParamType for ShaderParamTypeBool {
        type RustType = bool;

        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType) {
            gs_effect_set_bool(param, *value);
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind {
            ShaderParamTypeKind::Bool
        }
    }

    pub struct ShaderParamTypeFloat;
    impl ShaderParamType for ShaderParamTypeFloat {
        type RustType = f32;

        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType) {
            gs_effect_set_float(param, *value);
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind {
            ShaderParamTypeKind::Float
        }
    }

    pub struct ShaderParamTypeInt;
    impl ShaderParamType for ShaderParamTypeInt {
        type RustType = i32;

        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType) {
            gs_effect_set_int(param, *value);
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind {
            ShaderParamTypeKind::Int
        }
    }

    pub struct ShaderParamTypeVec2;
    impl ShaderParamType for ShaderParamTypeVec2 {
        type RustType = [f32; 2];

        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType) {
            let mut value = Vec2::new(value[0], value[1]);
            gs_effect_set_vec2(param, value.as_ptr());
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind {
            ShaderParamTypeKind::Vec2
        }
    }

    pub struct ShaderParamTypeVec3;
    impl ShaderParamType for ShaderParamTypeVec3 {
        type RustType = [f32; 3];

        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType) {
            let mut value = Vec3::new(value[0], value[1], value[2]);
            gs_effect_set_vec3(param, value.as_ptr());
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind {
            ShaderParamTypeKind::Vec3
        }
    }

    pub struct ShaderParamTypeVec4;
    impl ShaderParamType for ShaderParamTypeVec4 {
        type RustType = [f32; 4];

        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType) {
            let mut value = Vec4::new(value[0], value[1], value[2], value[3]);
            gs_effect_set_vec4(param, value.as_ptr());
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind {
            ShaderParamTypeKind::Vec4
        }
    }

    pub struct ShaderParamTypeIVec2;
    impl ShaderParamType for ShaderParamTypeIVec2 {
        type RustType = [i32; 2];

        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType) {
            gs_effect_set_val(
                param,
                value as *const _ as *const c_void,
                mem::size_of::<Self::RustType>() as size_t,
            );
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind {
            ShaderParamTypeKind::IVec2
        }
    }

    pub struct ShaderParamTypeIVec3;
    impl ShaderParamType for ShaderParamTypeIVec3 {
        type RustType = [i32; 3];

        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType) {
            gs_effect_set_val(
                param,
                value as *const _ as *const c_void,
                mem::size_of::<Self::RustType>() as size_t,
            );
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind {
            ShaderParamTypeKind::IVec3
        }
    }

    pub struct ShaderParamTypeIVec4;
    impl ShaderParamType for ShaderParamTypeIVec4 {
        type RustType = [i32; 4];

        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType) {
            gs_effect_set_val(
                param,
                value as *const _ as *const c_void,
                mem::size_of::<Self::RustType>() as size_t,
            );
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind {
            ShaderParamTypeKind::IVec4
        }
    }

    pub struct ShaderParamTypeMat4;
    impl ShaderParamType for ShaderParamTypeMat4 {
        type RustType = [[f32; 4]; 4];

        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType) {
            gs_effect_set_val(
                param,
                value as *const _ as *const c_void,
                mem::size_of::<Self::RustType>() as size_t,
            );
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind {
            ShaderParamTypeKind::Mat4
        }
    }

    pub struct ShaderParamTypeTexture;
    impl ShaderParamType for ShaderParamTypeTexture {
        type RustType = Texture;

        unsafe fn set_param_value(param: *mut gs_eparam_t, value: &Self::RustType) {
            gs_effect_set_texture(
                param,
                value.inner() as *mut gs_texture_t,
            );
        }

        unsafe fn get_param_value_default<'a>(param: *mut gs_eparam_t) -> &'a Self::RustType {
            // TODO: Consider changing abstractions to remove this panic using type safety
            panic!("Cannot access the value of a texture effect parameter.");
        }

        fn corresponding_enum_variant() -> ShaderParamTypeKind {
            ShaderParamTypeKind::Texture
        }
    }
}

pub use shader_param_types::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderParamTypeKind {
    Unknown,
    Bool,
    Float,
    Int,
    String,
    Vec2,
    Vec3,
    Vec4,
    IVec2,
    IVec3,
    IVec4,
    Mat4,
    Texture,
}

impl ShaderParamTypeKind {
    pub fn as_raw(&self) -> gs_shader_param_type {
        match self {
            ShaderParamTypeKind::Unknown => gs_shader_param_type_GS_SHADER_PARAM_UNKNOWN,
            ShaderParamTypeKind::Bool => gs_shader_param_type_GS_SHADER_PARAM_BOOL,
            ShaderParamTypeKind::Float => gs_shader_param_type_GS_SHADER_PARAM_FLOAT,
            ShaderParamTypeKind::Int => gs_shader_param_type_GS_SHADER_PARAM_INT,
            ShaderParamTypeKind::String => gs_shader_param_type_GS_SHADER_PARAM_STRING,
            ShaderParamTypeKind::Vec2 => gs_shader_param_type_GS_SHADER_PARAM_VEC2,
            ShaderParamTypeKind::Vec3 => gs_shader_param_type_GS_SHADER_PARAM_VEC3,
            ShaderParamTypeKind::Vec4 => gs_shader_param_type_GS_SHADER_PARAM_VEC4,
            ShaderParamTypeKind::IVec2 => gs_shader_param_type_GS_SHADER_PARAM_INT2,
            ShaderParamTypeKind::IVec3 => gs_shader_param_type_GS_SHADER_PARAM_INT3,
            ShaderParamTypeKind::IVec4 => gs_shader_param_type_GS_SHADER_PARAM_INT4,
            ShaderParamTypeKind::Mat4 => gs_shader_param_type_GS_SHADER_PARAM_MATRIX4X4,
            ShaderParamTypeKind::Texture => gs_shader_param_type_GS_SHADER_PARAM_TEXTURE,
        }
    }

    #[allow(non_upper_case_globals)]
    pub fn from_raw(param_type: gs_shader_param_type) -> Self {
        match param_type {
            gs_shader_param_type_GS_SHADER_PARAM_UNKNOWN => ShaderParamTypeKind::Unknown,
            gs_shader_param_type_GS_SHADER_PARAM_BOOL => ShaderParamTypeKind::Bool,
            gs_shader_param_type_GS_SHADER_PARAM_FLOAT => ShaderParamTypeKind::Float,
            gs_shader_param_type_GS_SHADER_PARAM_INT => ShaderParamTypeKind::Int,
            gs_shader_param_type_GS_SHADER_PARAM_STRING => ShaderParamTypeKind::String,
            gs_shader_param_type_GS_SHADER_PARAM_VEC2 => ShaderParamTypeKind::Vec2,
            gs_shader_param_type_GS_SHADER_PARAM_VEC3 => ShaderParamTypeKind::Vec3,
            gs_shader_param_type_GS_SHADER_PARAM_VEC4 => ShaderParamTypeKind::Vec4,
            gs_shader_param_type_GS_SHADER_PARAM_INT2 => ShaderParamTypeKind::IVec2,
            gs_shader_param_type_GS_SHADER_PARAM_INT3 => ShaderParamTypeKind::IVec3,
            gs_shader_param_type_GS_SHADER_PARAM_INT4 => ShaderParamTypeKind::IVec4,
            gs_shader_param_type_GS_SHADER_PARAM_MATRIX4X4 => ShaderParamTypeKind::Mat4,
            gs_shader_param_type_GS_SHADER_PARAM_TEXTURE => ShaderParamTypeKind::Texture,
            _ => panic!("Invalid param_type!"),
        }
    }
}

pub struct GraphicsEffect {
    raw: *mut gs_effect_t,
}

impl GraphicsEffect {
    pub fn from_effect_string<'a>(value: &CStr, name: &CStr, context: &'a GraphicsContext) -> Option<GraphicsContextDependentEnabled<'a, Self>> {
        unsafe {
            let raw = gs_effect_create(value.as_ptr(), name.as_ptr(), std::ptr::null_mut());

            if raw.is_null() {
                None
            } else {
                Some(ContextDependent::new(Self { raw }, context))
            }
        }
    }

    pub fn get_param_count(&self) -> usize {
        unsafe {
            gs_effect_get_num_params(self.raw as *const _) as usize
        }
    }

    pub fn get_param_by_index<'a>(
        self: &GraphicsContextDependentEnabled<'a, Self>,
        index: usize,
    ) -> Option<GraphicsContextDependentEnabled<'a, GraphicsEffectParam>> {
        unsafe {
            let pointer = gs_effect_get_param_by_idx(self.raw, index as size_t);
            if !pointer.is_null() {
                Some(GraphicsEffectParam::from_raw(pointer, self.context()))
            } else {
                None
            }
        }
    }

    pub fn get_param_by_name<'a>(
        self: &GraphicsContextDependentEnabled<'a, Self>,
        name: &CStr,
    ) -> Option<GraphicsContextDependentEnabled<'a, GraphicsEffectParam>> {
        unsafe {
            let pointer = gs_effect_get_param_by_name(self.raw, name.as_ptr());
            if !pointer.is_null() {
                Some(GraphicsEffectParam::from_raw(pointer, self.context()))
            } else {
                None
            }
        }
    }

    pub fn params_iter<'a, 'b>(
        self: &'b GraphicsContextDependentEnabled<'a, Self>,
    ) -> impl Iterator<Item=GraphicsContextDependentEnabled<'a, GraphicsEffectParam>> + 'b {
        struct EffectParamIterator<'a, 'b> {
            effect: &'b GraphicsContextDependentEnabled<'a, GraphicsEffect>,
            next_index: usize,
            len: usize,
        }

        impl<'a, 'b> Iterator for EffectParamIterator<'a, 'b> {
            type Item = GraphicsContextDependentEnabled<'a, GraphicsEffectParam>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.next_index < self.len {
                    let param = GraphicsEffect::get_param_by_index(self.effect, self.next_index)
                        .expect("An effect parameter went unexpectedly missing.");
                    self.next_index += 1;

                    Some(param)
                } else {
                    None
                }
            }
        }

        EffectParamIterator {
            len: self.get_param_count(),
            effect: self,
            next_index: 0,
        } as EffectParamIterator<'a, 'b>
    }

    /// # Safety
    /// Returns a mutable pointer to an effect which if modified could cause UB.
    pub unsafe fn as_ptr(&self) -> *mut gs_effect_t {
        self.raw
    }
}

impl Drop for GraphicsEffect {
    fn drop(&mut self) {
        unsafe {
            gs_effect_destroy(self.raw);
        }
    }
}

pub struct GraphicsEffectParam {
    raw: *mut gs_eparam_t,
    name: String,
    shader_type: ShaderParamTypeKind,
}

impl GraphicsEffectParam {
    /// # Safety
    /// Creates a GraphicsEffectParam from a mutable reference. This data could be modified
    /// somewhere else so this is UB.
    pub unsafe fn from_raw<'a>(raw: *mut gs_eparam_t, context: &'a GraphicsContext) -> GraphicsContextDependentEnabled<'a, Self> {
        let mut info = gs_effect_param_info::default();
        gs_effect_get_param_info(raw, &mut info);

        let shader_type = ShaderParamTypeKind::from_raw(info.type_);
        let name = CString::from(CStr::from_ptr(info.name))
            .into_string()
            .unwrap_or(String::from("{unknown-param-name}"));

        ContextDependent::new(
            Self {
                raw,
                shader_type,
                name,
            },
            context,
        )
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn param_type(&self) -> ShaderParamTypeKind {
        self.shader_type
    }

    pub fn downcast<'a, T: ShaderParamType>(self: GraphicsContextDependentEnabled<'a, Self>) -> Option<GraphicsContextDependentEnabled<'a, GraphicsEffectParamTyped<T>>> {
        if self.shader_type == <T as ShaderParamType>::corresponding_enum_variant() {
            Some(self.map(|inner| {
                GraphicsEffectParamTyped {
                    inner,
                    __marker: Default::default(),
                }
            }))
        } else {
            None
        }
    }
}

pub struct GraphicsEffectParamTyped<T: ShaderParamType> {
    pub inner: GraphicsEffectParam,
    __marker: std::marker::PhantomData<T>,
}

impl<T: ShaderParamType> GraphicsEffectParamTyped<T> {
    pub fn set_param_value(&mut self, value: &<T as ShaderParamType>::RustType) {
        unsafe {
            <T as ShaderParamType>::set_param_value(self.inner.raw, value);
        }
    }

    pub fn get_param_value_default<'a>(&'a self) -> &'a <T as ShaderParamType>::RustType {
        unsafe {
            <T as ShaderParamType>::get_param_value_default::<'a>(self.inner.raw)
        }
    }
}

impl GraphicsEffectParamTyped<ShaderParamTypeTexture> {
    pub fn set_next_sampler(
        &mut self,
        _context: &GraphicsContext,
        value: &mut GraphicsSamplerState,
    ) {
        unsafe {
            gs_effect_set_next_sampler(self.inner.raw, value.raw);
        }
    }
}

pub enum GraphicsAddressMode {
    Clamp,
    Wrap,
    Mirror,
    Border,
    MirrorOnce,
}

impl GraphicsAddressMode {
    pub fn as_raw(&self) -> gs_address_mode {
        match self {
            GraphicsAddressMode::Clamp => gs_address_mode_GS_ADDRESS_CLAMP,
            GraphicsAddressMode::Wrap => gs_address_mode_GS_ADDRESS_WRAP,
            GraphicsAddressMode::Mirror => gs_address_mode_GS_ADDRESS_MIRROR,
            GraphicsAddressMode::Border => gs_address_mode_GS_ADDRESS_BORDER,
            GraphicsAddressMode::MirrorOnce => gs_address_mode_GS_ADDRESS_MIRRORONCE,
        }
    }
}

pub enum GraphicsSampleFilter {
    Point,
    Linear,
    Anisotropic,
    MinMagPointMipLinear,
    MinPointMagLinearMipPoint,
    MinPointMagMipLinear,
    MinLinearMapMipPoint,
    MinLinearMagPointMipLinear,
    MinMagLinearMipPoint,
}

impl GraphicsSampleFilter {
    fn as_raw(&self) -> gs_sample_filter {
        match self {
            GraphicsSampleFilter::Point => gs_sample_filter_GS_FILTER_POINT,
            GraphicsSampleFilter::Linear => gs_sample_filter_GS_FILTER_LINEAR,
            GraphicsSampleFilter::Anisotropic => gs_sample_filter_GS_FILTER_ANISOTROPIC,
            GraphicsSampleFilter::MinMagPointMipLinear => {
                gs_sample_filter_GS_FILTER_MIN_MAG_POINT_MIP_LINEAR
            }
            GraphicsSampleFilter::MinPointMagLinearMipPoint => {
                gs_sample_filter_GS_FILTER_MIN_POINT_MAG_LINEAR_MIP_POINT
            }
            GraphicsSampleFilter::MinPointMagMipLinear => {
                gs_sample_filter_GS_FILTER_MIN_POINT_MAG_MIP_LINEAR
            }
            GraphicsSampleFilter::MinLinearMapMipPoint => {
                gs_sample_filter_GS_FILTER_MIN_LINEAR_MAG_MIP_POINT
            }
            GraphicsSampleFilter::MinLinearMagPointMipLinear => {
                gs_sample_filter_GS_FILTER_MIN_LINEAR_MAG_POINT_MIP_LINEAR
            }
            GraphicsSampleFilter::MinMagLinearMipPoint => {
                gs_sample_filter_GS_FILTER_MIN_MAG_LINEAR_MIP_POINT
            }
        }
    }
}

pub struct GraphicsSamplerInfo {
    info: gs_sampler_info,
}

impl GraphicsSamplerInfo {
    pub fn new<'a>(context: &'a GraphicsContext) -> GraphicsContextDependentEnabled<'a, Self> {
        ContextDependent::new(Self {
            info: gs_sampler_info {
                address_u: GraphicsAddressMode::Clamp.as_raw(),
                address_v: GraphicsAddressMode::Clamp.as_raw(),
                address_w: GraphicsAddressMode::Clamp.as_raw(),
                max_anisotropy: 0,
                border_color: 0,
                filter: GraphicsSampleFilter::Point.as_raw(),
            },
        }, context)
    }

    pub fn with_address_u(mut self, mode: GraphicsAddressMode) -> Self {
        self.info.address_u = mode.as_raw();
        self
    }

    pub fn with_address_v(mut self, mode: GraphicsAddressMode) -> Self {
        self.info.address_v = mode.as_raw();
        self
    }

    pub fn with_address_w(mut self, mode: GraphicsAddressMode) -> Self {
        self.info.address_w = mode.as_raw();
        self
    }

    pub fn with_filter(mut self, mode: GraphicsSampleFilter) -> Self {
        self.info.filter = mode.as_raw();
        self
    }
}

pub struct GraphicsSamplerState {
    raw: *mut gs_samplerstate_t,
}

impl<'a> From<GraphicsContextDependentEnabled<'a, GraphicsSamplerInfo>> for GraphicsContextDependentEnabled<'a, GraphicsSamplerState> {
    fn from(info: GraphicsContextDependentEnabled<'a, GraphicsSamplerInfo>) -> Self {
        unsafe {
            let raw = gs_samplerstate_create(&info.info);

            ContextDependent::new(
                GraphicsSamplerState {
                    raw,
                },
                info.context(),
            )
        }
    }
}

impl Drop for GraphicsSamplerState {
    fn drop(&mut self) {
        unsafe {
            gs_samplerstate_destroy(self.raw);
        }
    }
}

pub enum GraphicsAllowDirectRendering {
    NoDirectRendering,
    AllowDirectRendering,
}

impl GraphicsAllowDirectRendering {
    pub fn as_raw(&self) -> obs_allow_direct_render {
        match self {
            GraphicsAllowDirectRendering::NoDirectRendering => {
                obs_allow_direct_render_OBS_NO_DIRECT_RENDERING
            }
            GraphicsAllowDirectRendering::AllowDirectRendering => {
                obs_allow_direct_render_OBS_ALLOW_DIRECT_RENDERING
            }
        }
    }
}

/// A handle to the graphics context.
pub struct GraphicsContext {
    inner: *mut graphics_t,
    drop: bool,
}

impl Context for GraphicsContext {
    fn enter_once() -> Option<Self> {
        if Self::get_current().is_some() {
            return None;
        }

        unsafe {
            obs_enter_graphics();

            Self::get_current().map(|mut context| {
                context.drop = true;
                context
            })
        }
    }

    /// Certain callbacks will automatically be within the graphics context, such as:
    /// `obs_source_info.video_render`, the callbacks of `obs_display_add_draw_callback()`
    /// and `obs_add_main_render_callback()`.
    ///
    /// This function is useful to access the context.
    /// If access to the graphics context is required outside of these callbacks,
    /// use `Context::enter` to enter the context.
    fn get_current() -> Option<Self> {
        unsafe {
            let inner = gs_get_context();

            if inner == std::ptr::null_mut() {
                None
            } else {
                Some(Self {
                    inner,
                    drop: false,
                })
            }
        }
    }
}

impl GraphicsContext {
}

impl Drop for GraphicsContext {
    fn drop(&mut self) {
        if self.drop {
            unsafe {
                obs_leave_graphics();
            }
        }
    }
}

pub type GraphicsContextDependentEnabled<'a, T> = ContextDependent<T, GraphicsContext, Enabled<'a, GraphicsContext>>;
pub type GraphicsContextDependentDisabled<T> = ContextDependent<T, GraphicsContext, Disabled>;

macro_rules! vector_impls {
    ($($rust_name: ident, $name:ident => $($component:ident)*,)*) => (
        $(
        #[derive(Clone)]
        struct $rust_name {
            raw: $name,
        }

        impl $rust_name {
            fn new($( $component: f32, )*) -> Self {
                let mut v = Self {
                    raw: $name::default(),
                };
                v.set($($component,)*);
                v
            }

            #[inline]
            fn set(&mut self, $( $component: f32, )*) {
                $(
                    self.raw.__bindgen_anon_1.__bindgen_anon_1.$component = $component;
                )*
            }

            $(
                item! {
                    #[inline]
                    fn [<$component>](&self) -> f32 {
                        unsafe {
                            self.raw.__bindgen_anon_1.__bindgen_anon_1.$component
                        }
                    }
                }
            )*

            pub unsafe fn as_ptr(&mut self) -> *mut $name {
                &mut self.raw
            }
        }

        impl Default for $rust_name {
            fn default() -> Self {
                $(
                    let $component = 0.;
                )*
                Self::new($( $component, )*)
            }
        }
        )*
    );
}

vector_impls! {
    Vec2, vec2 => x y,
    Vec3, vec3 => x y z,
    Vec4, vec4 => x y z w,
}
