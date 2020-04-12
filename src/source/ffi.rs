use super::context::{ActiveContext, VideoRenderContext};
use super::properties::{Properties, SettingsContext};
use super::traits::*;
use super::{EnumActiveContext, EnumAllContext, SourceContext};
use std::ffi::c_void;
use std::os::raw::c_char;

use obs_sys::{
    gs_effect_t, obs_data_t, obs_properties, obs_properties_create, obs_source_audio_mix,
    obs_source_enum_proc_t, obs_source_t, size_t,
};

pub(crate) struct DataWrapper<D> {
    pub(crate) settings: Option<SettingsContext>,
    pub(crate) data: Option<D>,
}

impl<D> Default for DataWrapper<D> {
    fn default() -> Self {
        Self {
            settings: None,
            data: None,
        }
    }
}

impl<D> DataWrapper<D> {
    pub fn new(settings: SettingsContext) -> Self {
        Self {
            settings: Some(settings),
            data: None,
        }
    }
}

pub unsafe extern "C" fn get_name<D, F: GetNameSource<D>>(
    _type_data: *mut c_void,
) -> *const c_char {
    F::get_name().as_ptr()
}

pub unsafe extern "C" fn get_width<D, F: GetWidthSource<D>>(data: *mut c_void) -> u32 {
    let context = PluginContext::<D>::from(data);
    F::get_width(context)
}

pub unsafe extern "C" fn get_height<D, F: GetHeightSource<D>>(data: *mut c_void) -> u32 {
    let context = PluginContext::<D>::from(data);
    F::get_height(context)
}

pub unsafe extern "C" fn create_default_data<D>(
    _settings: *mut obs_data_t,
    _source: *mut obs_source_t,
) -> *mut c_void {
    let data = Box::new(DataWrapper::<D>::default());
    Box::into_raw(data) as *mut c_void
}

pub unsafe extern "C" fn create<D, F: CreatableSource<D>>(
    settings: *mut obs_data_t,
    source: *mut obs_source_t,
) -> *mut c_void {
    let settings = SettingsContext::from_raw(settings);
    let mut wrapper = DataWrapper::new(settings);

    let source = SourceContext { source };

    let data = F::create(wrapper.settings.as_mut().unwrap(), source);

    wrapper.data = Some(data);

    Box::into_raw(Box::new(wrapper)) as *mut c_void
}

pub unsafe extern "C" fn destroy<D>(data: *mut c_void) {
    let wrapper: Box<DataWrapper<D>> = Box::from_raw(data as *mut DataWrapper<D>);
    drop(wrapper);
}

pub unsafe extern "C" fn update<D, F: UpdateSource<D>>(
    data: *mut c_void,
    settings: *mut obs_data_t,
) {
    let context = PluginContext::<D>::from(data);
    let mut active = ActiveContext::default();
    F::update(context, &mut active);
}

pub unsafe extern "C" fn video_render<D, F: VideoRenderSource<D>>(
    data: *mut ::std::os::raw::c_void,
    _effect: *mut gs_effect_t,
) {
    let context = PluginContext::<D>::from(data);
    let mut active = ActiveContext::default();
    let mut render = VideoRenderContext::default();
    F::video_render(context, &mut active, &mut render);
}

pub unsafe extern "C" fn audio_render<D, F: AudioRenderSource<D>>(
    data: *mut ::std::os::raw::c_void,
    _ts_out: *mut u64,
    _audio_output: *mut obs_source_audio_mix,
    _mixers: u32,
    _channels: size_t,
    _sample_rate: size_t,
) -> bool {
    let context = PluginContext::<D>::from(data);
    let mut active = ActiveContext::default();
    F::audio_render(context, &mut active);
    // TODO: understand what this bool is
    true
}

pub unsafe extern "C" fn get_properties<D, F: GetPropertiesSource<D>>(
    data: *mut ::std::os::raw::c_void,
) -> *mut obs_properties {
    let context = PluginContext::<D>::from(data);
    let properties = F::get_properties(context);
    let properties_ptr = properties.as_raw();

    // Ensure not to free the `properties_ptr` before returning it
    std::mem::forget(properties);

    properties_ptr
}

pub unsafe extern "C" fn enum_active_sources<D, F: EnumActiveSource<D>>(
    data: *mut ::std::os::raw::c_void,
    _enum_callback: obs_source_enum_proc_t,
    _param: *mut ::std::os::raw::c_void,
) {
    let context = PluginContext::<D>::from(data);
    let enum_context = EnumActiveContext {};
    F::enum_active_sources(context, &enum_context);
}

pub unsafe extern "C" fn enum_all_sources<D, F: EnumAllSource<D>>(
    data: *mut ::std::os::raw::c_void,
    _enum_callback: obs_source_enum_proc_t,
    _param: *mut ::std::os::raw::c_void,
) {
    let context = PluginContext::<D>::from(data);
    let enum_context = EnumAllContext {};
    F::enum_all_sources(context, &enum_context);
}

pub unsafe extern "C" fn transition_start<D, F: TransitionStartSource<D>>(
    data: *mut ::std::os::raw::c_void,
) {
    let context = PluginContext::<D>::from(data);
    F::transition_start(context);
}

pub unsafe extern "C" fn transition_stop<D, F: TransitionStopSource<D>>(
    data: *mut ::std::os::raw::c_void,
) {
    let context = PluginContext::<D>::from(data);
    F::transition_stop(context);
}

pub unsafe extern "C" fn video_tick<D, F: VideoTickSource<D>>(
    data: *mut ::std::os::raw::c_void,
    seconds: f32,
) {
    let context = PluginContext::<D>::from(data);
    F::video_tick(context, seconds);
}
