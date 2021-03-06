use super::properties::{Properties, SettingsContext};
use super::{EnumActiveContext, EnumAllContext, SourceContext, SourceType};
use std::ffi::CStr;
use std::ffi::c_void;
use crate::source::ffi::DataWrapper;
use crate::graphics::*;

pub struct PluginContext<'a, D> {
    data_wrapper: &'a mut DataWrapper<D>,
}

impl<'a, D> PluginContext<'a, D> {
    pub(crate) unsafe fn from(data: *mut c_void) -> Self {
        let wrapper: &mut DataWrapper<D> = &mut *(data as *mut DataWrapper<D>);

        Self {
            data_wrapper: wrapper
        }
    }

    pub fn data(&self) -> &Option<D> {
        &self.data_wrapper.data
    }

    pub fn data_mut(&mut self) -> &mut Option<D> {
        &mut self.data_wrapper.data
    }

    pub fn settings(&self) -> &SettingsContext {
        self.data_wrapper.settings.as_ref()
            .expect("Settings were not initialized.")
    }

    pub fn settings_mut(&mut self) -> &mut SettingsContext {
        self.data_wrapper.settings.as_mut()
            .expect("Settings were not initialized.")
    }

    pub fn data_settings_mut(&mut self) -> (&mut Option<D>, &mut SettingsContext) {
        (
            &mut self.data_wrapper.data,
            self.data_wrapper.settings.as_mut()
                .expect("Settings were not initialized."),
        )
    }
}

pub trait Sourceable {
    fn get_id() -> &'static CStr;
    fn get_type() -> SourceType;
}

pub trait GetNameSource<D> {
    fn get_name() -> &'static CStr;
}

pub trait GetWidthSource<D> {
    fn get_width(context: PluginContext<D>) -> u32;
}

pub trait GetHeightSource<D> {
    fn get_height(context: PluginContext<D>) -> u32;
}

pub trait CreatableSource<D> {
    fn create(settings: &mut SettingsContext, source: SourceContext) -> D;
}

pub trait UpdateSource<D> {
    fn update(context: PluginContext<D>);
}

pub trait VideoRenderSource<D> {
    fn video_render(
        context: PluginContext<D>,
        graphics_context: &mut GraphicsContext,
    );
}

pub trait AudioRenderSource<D> {
    fn audio_render(context: PluginContext<D>);
}

pub trait GetPropertiesSource<D> {
    fn get_properties(context: PluginContext<D>) -> Properties;
}

pub trait VideoTickSource<D> {
    fn video_tick(context: PluginContext<D>, seconds: f32);
}

pub trait ActivateSource<D> {
    fn activate(context: PluginContext<D>);
}

pub trait DeactivateSource<D> {
    fn deactivate(context: PluginContext<D>);
}

pub trait ShowSource<D> {
    fn show(context: PluginContext<D>);
}

pub trait HideSource<D> {
    fn hide(context: PluginContext<D>);
}

pub trait EnumActiveSource<D> {
    fn enum_active_sources(context: PluginContext<D>, context: &EnumActiveContext);
}

pub trait EnumAllSource<D> {
    fn enum_all_sources(context: PluginContext<D>, context: &EnumAllContext);
}

pub trait TransitionStartSource<D> {
    fn transition_start(context: PluginContext<D>);
}

pub trait TransitionStopSource<D> {
    fn transition_stop(context: PluginContext<D>);
}
