mod server;

use server::{Server, WindowSnapshot};
use obs_wrapper::{graphics::*, obs_register_module, prelude::*, source::*};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::ffi::{CStr, CString};

enum FilterMessage {
    CloseConnection,
}

enum ServerMessage {
    Snapshot(WindowSnapshot),
}

struct Data {
    source: SourceContext,
    effect: GraphicsEffect,

    mul_val: GraphicsEffectParamTyped<ShaderParamTypeVec2>,
    add_val: GraphicsEffectParamTyped<ShaderParamTypeVec2>,
    image: GraphicsEffectParamTyped<ShaderParamTypeTexture>,

    sampler: GraphicsSamplerState,

    send: Sender<FilterMessage>,
    receive: Receiver<ServerMessage>,

    current: [f32; 2],
    from: [f32; 2],
    target: [f32; 2],

    animation_time: f64,

    current_zoom: f64,
    from_zoom: f64,
    target_zoom: f64,
    internal_zoom: f64,

    progress: f64,

    screen_width: u32,
    screen_height: u32,
    screen_x: u32,
    screen_y: u32,

    property_zoom: Option<Property<PropertyDescriptorF64>>,
    property_screen_x: Option<Property<PropertyDescriptorI32>>,
    property_screen_y: Option<Property<PropertyDescriptorI32>>,
    property_screen_width: Option<Property<PropertyDescriptorI32>>,
    property_screen_height: Option<Property<PropertyDescriptorI32>>,
    property_animation_time: Option<Property<PropertyDescriptorF64>>,
}

impl Drop for Data {
    fn drop(&mut self) {
        self.send.send(FilterMessage::CloseConnection).unwrap_or(());
    }
}

struct ScrollFocusFilter {
    context: ModuleContext,
}

impl Sourceable for ScrollFocusFilter {
    fn get_id() -> &'static CStr {
        cstr!("scroll_focus_filter")
    }
    fn get_type() -> SourceType {
        SourceType::FILTER
    }
}

impl GetNameSource<Data> for ScrollFocusFilter {
    fn get_name() -> &'static CStr {
        cstr!("Scroll Focus Filter")
    }
}

impl GetPropertiesSource<Data> for ScrollFocusFilter {
    fn get_properties(_data: &mut Option<Data>, properties: &mut Properties) {
        let data = _data.as_mut().unwrap();
        data.property_zoom = Some(properties.add_property(
            "zoom".to_string(),
            "Amount to zoom in window".to_string(),
            PropertyDescriptorF64 {
                min: 1.0,
                max: 5.0,
                step: 0.001,
                slider: true,
            },
        ));
        data.property_screen_x = Some(properties.add_property(
            "screen_x".to_string(),
            "Offset relative to top left screen - x".to_string(),
            PropertyDescriptorI32 {
                min: 0,
                max: 3840 * 3,
                step: 1,
                slider: false,
            },
        ));
        data.property_screen_y = Some(properties.add_property(
            "screen_y".to_string(),
            "Offset relative to top left screen - y".to_string(),
            PropertyDescriptorI32 {
                min: 0,
                max: 3840 * 3,
                step: 1,
                slider: false,
            },
        ));
        data.property_screen_width = Some(properties.add_property(
            "screen_width".to_string(),
            "Screen width".to_string(),
            PropertyDescriptorI32 {
                min: 1,
                max: 3840 * 3,
                step: 1,
                slider: false,
            },
        ));
        data.property_screen_height = Some(properties.add_property(
            "screen_height".to_string(),
            "Screen height".to_string(),
            PropertyDescriptorI32 {
                min: 1,
                max: 3840 * 3,
                step: 1,
                slider: false,
            },
        ));
        data.property_animation_time = Some(properties.add_property(
            "animation_time".to_string(),
            "Animation Time (s)".to_string(),
            PropertyDescriptorF64 {
                min: 0.3,
                max: 10.,
                step: 0.001,
                slider: false,
            },
        ));
    }
}

fn smooth_step(x: f32) -> f32 {
    let t = ((x / 1.).max(0.)).min(1.);
    t * t * (3. - 2. * t)
}

impl VideoTickSource<Data> for ScrollFocusFilter {
    fn video_tick(data: &mut Option<Data>, seconds: f32) {
        if let Some(data) = data {
            for message in data.receive.try_iter() {
                match message {
                    ServerMessage::Snapshot(snapshot) => {
                        let window_zoom = ((snapshot.width / (data.screen_width as f32))
                            .max(snapshot.height / (data.screen_height as f32))
                            as f64
                            + 0.1)
                            .max(data.internal_zoom)
                            .min(1.);

                        if snapshot.x > (data.screen_width + data.screen_x) as f32
                            || snapshot.x < data.screen_x as f32
                            || snapshot.y < data.screen_y as f32
                            || snapshot.y > (data.screen_height + data.screen_y) as f32
                        {
                            if data.target_zoom != 1.
                                && data.target[0] != 0.
                                && data.target[1] != 0.
                            {
                                data.progress = 0.;
                                data.from_zoom = data.current_zoom;
                                data.target_zoom = 1.;

                                data.from = data.current;
                                data.target = [0.0, 0.0];
                            }
                        } else {
                            let x = (snapshot.x + (snapshot.width / 2.) - (data.screen_x as f32))
                                / (data.screen_width as f32);
                            let y = (snapshot.y + (snapshot.height / 2.) - (data.screen_y as f32))
                                / (data.screen_height as f32);

                            let target_x = (x - (0.5 * window_zoom as f32))
                                .min(1. - window_zoom as f32)
                                .max(0.);

                            let target_y = (y - (0.5 * window_zoom as f32))
                                .min(1. - window_zoom as f32)
                                .max(0.);

                            if (target_y - data.target[1]).abs() > 0.001
                                || (target_x - data.target[0]).abs() > 0.001
                                || (window_zoom - data.target_zoom).abs() > 0.001
                            {
                                data.progress = 0.;

                                data.from_zoom = data.current_zoom;
                                data.target_zoom = window_zoom;

                                data.from = data.current;
                                data.target = [target_x, target_y];
                            }
                        }
                    }
                }
            }

            data.progress = (data.progress + seconds as f64 / data.animation_time).min(1.);

            let adjusted_progress = smooth_step(data.progress as f32);

            data.current = [
                data.from[0] + (data.target[0] - data.from[0]) * adjusted_progress,
                data.from[1] + (data.target[1] - data.from[1]) * adjusted_progress,
            ];

            data.current_zoom =
                data.from_zoom + (data.target_zoom - data.from_zoom) * adjusted_progress as f64;
        }
    }
}

impl VideoRenderSource<Data> for ScrollFocusFilter {
    fn video_render(
        data: &mut Option<Data>,
        _context: &mut ActiveContext,
        render: &mut VideoRenderContext,
    ) {
        if let Some(data) = data {
            let effect = &mut data.effect;
            let source = &mut data.source;
            let param_add = &mut data.add_val;
            let param_mul = &mut data.mul_val;
            let image = &mut data.image;
            let sampler = &mut data.sampler;

            let current = &mut data.current;

            let zoom = data.current_zoom as f32;

            let mut cx: u32 = 1;
            let mut cy: u32 = 1;

            source.do_with_target(|target| {
                cx = target.get_base_width();
                cy = target.get_base_height();
            });

            source.process_filter(
                render,
                effect,
                (cx, cy),
                GraphicsColorFormat::RGBA,
                GraphicsAllowDirectRendering::NoDirectRendering,
                |context, _effect| {
                    param_add.set_param_value(*current);
                    param_mul.set_param_value([zoom, zoom]);
                    image.set_next_sampler(context, sampler);
                },
            );
        }
    }
}

impl CreatableSource<Data> for ScrollFocusFilter {
    fn create(settings: &mut SettingsContext, mut source: SourceContext) -> Data {
        let effect_string = CString::new(include_str!("./crop_filter.effect")).unwrap();
        if let Some(mut effect) = GraphicsEffect::from_effect_string(
            effect_string.as_c_str(),
            cstr!("crop_filter.effect"),
        ) {
            let param_image = effect.get_effect_param_by_name(cstr!("image"));
            let param_add_val = effect.get_effect_param_by_name(cstr!("add_val"));
            let param_mul_val = effect.get_effect_param_by_name(cstr!("mul_val"));

            if param_image.is_none() || param_add_val.is_none() || param_mul_val.is_none() {
                panic!("Failed to find correct effect params!");
            }

            let param_image = param_image.unwrap().downcast::<ShaderParamTypeTexture>().unwrap();
            let param_add_val = param_add_val.unwrap().downcast::<ShaderParamTypeVec2>().unwrap();
            let param_mul_val = param_mul_val.unwrap().downcast::<ShaderParamTypeVec2>().unwrap();

            let zoom = 1.0;
            let screen_width = 1920;
            let screen_height = 1080;
            let screen_x = 0;
            let screen_y = 0;
            let animation_time = 0.3;

            let sampler = GraphicsSamplerState::from(GraphicsSamplerInfo::default());

            let (send_filter, receive_filter) = unbounded::<FilterMessage>();
            let (send_server, receive_server) = unbounded::<ServerMessage>();

            std::thread::spawn(move || {
                let mut server = Server::new().unwrap();

                loop {
                    if let Some(snapshot) = server.wait_for_event() {
                        send_server
                            .send(ServerMessage::Snapshot(snapshot))
                            .unwrap_or(());
                    }

                    if let Ok(msg) = receive_filter.try_recv() {
                        match msg {
                            FilterMessage::CloseConnection => {
                                return;
                            }
                        }
                    }
                }
            });

            source.update_source_settings(settings);

            return Data {
                source,
                effect,
                add_val: param_add_val,
                mul_val: param_mul_val,
                image: param_image,

                sampler,

                animation_time,

                current_zoom: zoom,
                from_zoom: zoom,
                target_zoom: zoom,
                internal_zoom: zoom,

                send: send_filter,
                receive: receive_server,

                current: [0.0, 0.0],
                from: [0.0, 0.0],
                target: [0.0, 0.0],

                progress: 1.,

                screen_height,
                screen_width,
                screen_x,
                screen_y,

                property_zoom: None,
                property_screen_x: None,
                property_screen_y: None,
                property_screen_width: None,
                property_screen_height: None,
                property_animation_time: None,
            };

            panic!("Failed to find correct effect params!");
        } else {
            panic!("Could not load crop filter effect!");
        }
    }
}

impl UpdateSource<Data> for ScrollFocusFilter {
    fn update(
        data: &mut Option<Data>,
        settings: &mut SettingsContext,
        _context: &mut ActiveContext,
    ) {
        if let Some(data) = data {
            if let Some(zoom) = data.property_zoom.as_ref().map(|property| settings.get_property_value(property)) {
                data.from_zoom = data.current_zoom;
                data.internal_zoom = 1. / zoom;
                data.target_zoom = 1. / zoom;
            }

            if let Some(screen_width) = data.property_screen_width.as_ref().map(|property| settings.get_property_value(property)) {
                data.screen_width = screen_width as u32;
            }

            if let Some(screen_height) = data.property_screen_height.as_ref().map(|property| settings.get_property_value(property)) {
                data.screen_height = screen_height as u32;
            }

            if let Some(screen_x) = data.property_screen_x.as_ref().map(|property| settings.get_property_value(property)) {
                data.screen_x = screen_x as u32;
            }

            if let Some(screen_y) = data.property_screen_y.as_ref().map(|property| settings.get_property_value(property)) {
                data.screen_y = screen_y as u32;
            }

            if let Some(animation_time) = data.property_animation_time.as_ref().map(|property| settings.get_property_value(property)) {
                data.animation_time = animation_time;
            }
        }
    }
}

impl Module for ScrollFocusFilter {
    fn new(context: ModuleContext) -> Self {
        Self { context }
    }
    fn get_ctx(&self) -> &ModuleContext {
        &self.context
    }

    fn load(&mut self, load_context: &mut LoadContext) -> bool {
        let source = load_context
            .create_source_builder::<ScrollFocusFilter, Data>()
            .enable_get_name()
            .enable_create()
            .enable_get_properties()
            .enable_update()
            .enable_video_render()
            .enable_video_tick()
            .build();

        load_context.register_source(source);

        true
    }

    fn description() -> &'static CStr {
        cstr!("A filter that focused the currently focused Xorg window.")
    }
    fn name() -> &'static CStr {
        cstr!("Scroll Focus Filter")
    }
    fn author() -> &'static CStr {
        cstr!("Bennett Hardwick")
    }
}

obs_register_module!(ScrollFocusFilter);
