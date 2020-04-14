use std::cell::RefCell;
use obs_sys::{graphics_t, gs_get_context, obs_enter_graphics, obs_leave_graphics};
use crate::context::*;

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

/// A context used to store source filter data to be submitted at the end of the processing.
pub struct FilterContext {
    graphics: GraphicsContext,
    data_entries: RefCell<Vec<Vec<u8>>>,
}

impl FilterContext {
    /// Stores data to be used at the end of filter processing.
    /// Ensure that the type you are converting to `&[u8]` does not need to be `Drop::drop`ped.
    pub unsafe fn store_until_end_of_processing(&self, data: &[u8]) -> *const u8 {
        let mut data_entries = self.data_entries.borrow_mut();
        let entry_index = data_entries.len();

        data_entries.push(Vec::from(data));

        data_entries[entry_index].as_ptr()
    }

    pub fn graphics(&self) -> &GraphicsContext {
        &self.graphics
    }
}

impl From<GraphicsContext> for FilterContext {
    fn from(graphics: GraphicsContext) -> Self {
        Self {
            graphics,
            data_entries: RefCell::new(Vec::new()),
        }
    }
}

impl Context for FilterContext {
    fn enter_once() -> Option<Self> {
        GraphicsContext::enter_once().map(From::from)
    }

    fn get_current() -> Option<Self> {
        GraphicsContext::get_current().map(From::from)
    }
}

impl ContextCarrier<GraphicsContext> for FilterContext {
    fn context(&self) -> &GraphicsContext {
        self.graphics()
    }
}

pub type FilterContextDependentEnabled<'a, T> = ContextDependent<T, FilterContext, Enabled<'a, FilterContext>>;
pub type FilterContextDependentDisabled<T> = ContextDependent<T, FilterContext, Disabled>;
