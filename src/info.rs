use std::mem::MaybeUninit;
use obs_sys::{obs_video_info, obs_get_video_info, obs_audio_info, obs_get_audio_info};
use crate::audio::SpeakerLayoutKind;

pub struct ObsVideoInfo {
    inner: obs_video_info,

    /* Members of `obs_video_info`:
    pub graphics_module: *const ::std::os::raw::c_char,
    pub fps_num: u32,
    pub fps_den: u32,
    pub base_width: u32,
    pub base_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub output_format: video_format,
    pub adapter: u32,
    pub gpu_conversion: bool,
    pub colorspace: video_colorspace,
    pub range: video_range_type,
    pub scale_type: obs_scale_type,
     */
}

impl ObsVideoInfo {
    pub fn get() -> Option<Self> {
        unsafe {
            let mut inner = MaybeUninit::<obs_video_info>::uninit();

            if !obs_get_video_info(inner.as_mut_ptr()) {
                None
            } else {
                Some(Self {
                    inner: inner.assume_init(),
                })
            }
        }
    }

    pub fn framerate(&self) -> FramesPerSecond {
        FramesPerSecond {
            numerator: self.inner.fps_num,
            denominator: self.inner.fps_den,
        }
    }

    pub fn base_dimensions(&self) -> [u32; 2] {
        [self.inner.base_width, self.inner.base_height]
    }

    pub fn output_dimensions(&self) -> [u32; 2] {
        [self.inner.output_width, self.inner.output_height]
    }

    // TODO implement the rest of the getters
}

pub struct ObsAudioInfo {
    inner: obs_audio_info,

    /* Members of `obs_audio_info`:
    pub samples_per_sec: u32,
    pub speakers: speaker_layout,
     */
}

impl ObsAudioInfo {
    pub fn get() -> Option<Self> {
        unsafe {
            let mut inner = MaybeUninit::<obs_audio_info>::uninit();

            if !obs_get_audio_info(inner.as_mut_ptr()) {
                None
            } else {
                Some(Self {
                    inner: inner.assume_init(),
                })
            }
        }
    }

    pub fn samples_per_second(&self) -> u32 {
        self.inner.samples_per_sec
    }

    pub fn speaker_layout(&self) -> SpeakerLayoutKind {
        SpeakerLayoutKind::from_raw(self.inner.speakers)
    }
}

pub struct FramesPerSecond {
    pub numerator: u32,
    pub denominator: u32,
}

impl FramesPerSecond {
    pub fn as_f32(&self) -> f32 {
        self.numerator as f32 / self.denominator as f32
    }

    pub fn as_f64(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}
