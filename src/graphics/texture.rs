use std::ffi::{c_void, CString};
use std::path::Path;
use crate::context::*;
use crate::graphics::*;
use obs_sys::{
    size_t,
    gs_texture_t,
    gs_copy_texture,
    gs_texture_create,
    gs_texture_create_from_file,
    gs_texture_destroy,
    gs_texture_get_width,
    gs_texture_get_height,
    gs_texture_get_color_format,
    gs_texture_get_obj,
    gs_color_format,
    gs_color_format_GS_A8,
    gs_color_format_GS_R8,
    gs_color_format_GS_RGBA,
    gs_color_format_GS_BGRX,
    gs_color_format_GS_BGRA,
    gs_color_format_GS_R10G10B10A2,
    gs_color_format_GS_RGBA16,
    gs_color_format_GS_R16,
    gs_color_format_GS_RGBA16F,
    gs_color_format_GS_RGBA32F,
    gs_color_format_GS_RG16F,
    gs_color_format_GS_RG32F,
    gs_color_format_GS_R16F,
    gs_color_format_GS_R32F,
    gs_color_format_GS_DXT1,
    gs_color_format_GS_DXT3,
    gs_color_format_GS_DXT5,
    gs_color_format_GS_R8G8,
    gs_color_format_GS_UNKNOWN,
    GS_BUILD_MIPMAPS,
    GS_DYNAMIC,
    GS_RENDER_TARGET,
    GS_GL_DUMMYTEX,
    GS_DUP_BUFFER,
    GS_SHARED_TEX,
    GS_SHARED_KM_TEX,
};

macro_rules! define_color_formats {
    {
        $(
            $binding:ident, $name:ident, $bytes_per_pixel:expr
        );*$(;)?
    } => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum ColorFormatKind {
            Unknown,
            $(
                $name
            ),*
        }

        impl ColorFormatKind {
            pub fn get_pixel_size_in_bytes(&self) -> usize {
                use ColorFormatKind::*;

                match self {
                    Unknown => 0,
                    $(
                        $name => $bytes_per_pixel,
                    )*
                }
            }

            pub fn from_raw(raw: gs_color_format) -> Self {
                use ColorFormatKind::*;

                #[allow(non_upper_case_globals)]
                match raw {
                    gs_color_format_GS_UNKNOWN => Unknown,
                    $(
                        $binding => $name,
                    )*
                    _ => Unknown,
                }
            }

            pub fn into_raw(self) -> gs_color_format {
                use ColorFormatKind::*;

                match self {
                    Unknown => gs_color_format_GS_UNKNOWN,
                    $(
                        $name => $binding,
                    )*
                }
            }
        }
    }
}

define_color_formats! {
    gs_color_format_GS_A8,          A8,          1;
    gs_color_format_GS_R8,          R8,          1;
    gs_color_format_GS_RGBA,        RGBA,        4;
    gs_color_format_GS_BGRX,        BGRX,        4;
    gs_color_format_GS_BGRA,        BGRA,        4;
    gs_color_format_GS_R10G10B10A2, R10G10B10A2, 4;
    gs_color_format_GS_RGBA16,      RGBA16,      8;
    gs_color_format_GS_R16,         R16,         2;
    gs_color_format_GS_RGBA16F,     RGBA16F,     8;
    gs_color_format_GS_RGBA32F,     RGBA32F,    16;
    gs_color_format_GS_RG16F,       RG16F,       4;
    gs_color_format_GS_RG32F,       RG32F,       8;
    gs_color_format_GS_R16F,        R16F,        2;
    gs_color_format_GS_R32F,        R32F,        4;
    gs_color_format_GS_DXT1,        DXT1,        0; // FIXME
    gs_color_format_GS_DXT3,        DXT3,        0; // FIXME
    gs_color_format_GS_DXT5,        DXT5,        0; // FIXME
    gs_color_format_GS_R8G8,        R8G8,        2;
}

pub const TEXTURE_FLAG_BUILD_MIPMAPS: u32 = GS_BUILD_MIPMAPS;
pub const TEXTURE_FLAG_DYNAMIC: u32 = GS_DYNAMIC;
pub const TEXTURE_FLAG_RENDER_TARGET: u32 = GS_RENDER_TARGET;
pub const TEXTURE_FLAG_GL_DUMMYTEX: u32 = GS_GL_DUMMYTEX;
pub const TEXTURE_FLAG_DUP_BUFFER: u32 = GS_DUP_BUFFER;
pub const TEXTURE_FLAG_SHARED_TEX: u32 = GS_SHARED_TEX;
pub const TEXTURE_FLAG_SHARED_KM_TEX: u32 = GS_SHARED_KM_TEX;

#[derive(Debug)]
pub struct Texture {
    inner: *mut gs_texture_t,
    flags: u32,
}

impl<'a> Clone for GraphicsContextDependentEnabled<'a, Texture> {
    fn clone(&self) -> Self {
        let dimensions = self.get_dimensions();
        let color_format = self.get_color_format();
        let bytes = dimensions[0] * dimensions[1] * color_format.get_pixel_size_in_bytes();
        let zero_data = vec![0; bytes];
        let mut cloned = Texture::new(dimensions, color_format, &[&zero_data], self.flags, self.context());

        self.copy_to(&mut cloned);

        cloned
    }
}

unsafe impl Send for Texture {}
unsafe impl Sync for Texture {}

impl DefaultInContext<GraphicsContext> for Texture {
    fn default_in_context<'a>(context: &'a GraphicsContext) -> GraphicsContextDependentEnabled<Self> {
        Self::new_dummy(context)
    }
}

impl Texture {
    pub unsafe fn from_raw(raw: *mut gs_texture_t, flags: u32) -> Self {
        Self {
            inner: raw,
            flags,
        }
    }

    pub fn new_dummy(context: &GraphicsContext) -> GraphicsContextDependentEnabled<Self> {
        let dimensions = [1, 1];
        let color_format = ColorFormatKind::RGBA;
        let bytes = dimensions[0] * dimensions[1] * color_format.get_pixel_size_in_bytes();
        let zero_data = vec![0; bytes];

        Self::new(dimensions, color_format, &[&zero_data], 0, context)
    }

    /// For flags, see constants defined in this module
    pub fn new<'a>(dimensions: [usize; 2], color_format: ColorFormatKind, levels: &[&[u8]], flags: u32, context: &'a GraphicsContext) -> GraphicsContextDependentEnabled<'a, Self> {
        let mut level_ptrs = levels.iter().map(|level_ref| {
            level_ref.as_ptr()
        }).collect::<Vec<_>>();

        // FIXME Add data size checks

        unsafe {
            let inner = gs_texture_create(
                dimensions[0] as u32,
                dimensions[1] as u32,
                color_format.into_raw(),
                levels.len() as u32,
                level_ptrs.as_mut_ptr(),
                flags,
            );

            if inner == std::ptr::null_mut() {
                panic!("An error occurred while creating a texture.");
            }

            ContextDependent::new(
                Self::from_raw(inner, flags),
                context,
            )
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Option<Self> {
        let path_string = path.as_ref().to_string_lossy();
        let path_string_c = CString::new(path_string.as_ref()).expect("Path is not a valid C String.");

        unsafe {
            let inner = gs_texture_create_from_file(path_string_c.as_ptr());

            if inner == std::ptr::null_mut() {
                None
            } else {
                Some(Self::from_raw(inner, 0))
            }
        }
    }

    pub fn get_dimensions(&self) -> [usize; 2] {
        unsafe {
            [
                gs_texture_get_width(self.inner) as usize,
                gs_texture_get_height(self.inner) as usize,
            ]
        }
    }

    pub fn get_color_format(&self) -> ColorFormatKind {
        unsafe {
            ColorFormatKind::from_raw(gs_texture_get_color_format(self.inner))
        }
    }

    pub fn get_interface_specific_object(&mut self) -> *mut c_void {
        unsafe {
            gs_texture_get_obj(self.inner)
        }
    }

    pub fn inner(&self) -> *const gs_texture_t {
        self.inner as *const _
    }

    pub fn inner_mut(&mut self) -> *mut gs_texture_t {
        self.inner
    }

    pub fn copy_to(&self, dst: &mut Texture) {
        unsafe {
            gs_copy_texture(dst.inner, self.inner)
        }
    }

    // TODO:
    // pub fn gs_copy_texture(dst: *mut gs_texture_t, src: *mut gs_texture_t);
    // pub fn gs_copy_texture_region(
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gs_texture_destroy(self.inner);
        }
    }
}
