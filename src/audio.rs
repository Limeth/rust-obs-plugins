use obs_sys::{
    audio_t, obs_get_audio, audio_output_connect, audio_output_disconnect, audio_data,
    audio_output_active, audio_output_get_block_size, audio_output_get_planes,
    audio_output_get_channels, audio_output_get_sample_rate, audio_output_get_info,
    audio_output_info, audio_format,
    audio_format_AUDIO_FORMAT_UNKNOWN,
    audio_format_AUDIO_FORMAT_U8BIT,
    audio_format_AUDIO_FORMAT_16BIT,
    audio_format_AUDIO_FORMAT_32BIT,
    audio_format_AUDIO_FORMAT_FLOAT,
    audio_format_AUDIO_FORMAT_U8BIT_PLANAR,
    audio_format_AUDIO_FORMAT_16BIT_PLANAR,
    audio_format_AUDIO_FORMAT_32BIT_PLANAR,
    audio_format_AUDIO_FORMAT_FLOAT_PLANAR,
    speaker_layout,
    speaker_layout_SPEAKERS_UNKNOWN,
    speaker_layout_SPEAKERS_MONO,
    speaker_layout_SPEAKERS_STEREO,
    speaker_layout_SPEAKERS_2POINT1,
    speaker_layout_SPEAKERS_4POINT0,
    speaker_layout_SPEAKERS_4POINT1,
    speaker_layout_SPEAKERS_5POINT1,
    speaker_layout_SPEAKERS_7POINT1,
};
use std::ptr::null_mut;
use std::os::raw::c_void;
use std::ffi::CStr;
use crate::util::*;

type size_t = ::std::os::raw::c_ulong;

pub struct AudioOutput {
    mix_index: usize,
    callback_ptr: *mut AudioOutputCallback,
}

unsafe impl Send for AudioOutput {}
unsafe impl Sync for AudioOutput {}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        unsafe {
            audio_output_disconnect(
                Audio::get().inner,
                self.mix_index as size_t, // Mix index to get the raw audio from
                Some(global_audio_output_callback),
                self.callback_ptr as *mut _,
            );

            std::mem::drop(Box::from_raw(self.callback_ptr as *mut _));
        }
    }
}

pub struct SampleIterator<'a, T: AudioFormat> {
    audio_data: AudioData<'a, T>,
    next_frame: usize,
    // All following values in bytes
    plane: usize,
    offset: usize,
    stride: usize,
}

impl<'a, T: AudioFormat> SampleIterator<'a, T> {
    pub fn new(audio_data: &AudioData<'a, T>, channel: usize) -> Option<Self> {
        let info = &audio_data.info;
        let format = info.format();
        let plane = if format.is_planar() {
            channel
        } else {
            0
        };

        let data = unsafe { &*audio_data.inner };

        if data.data[plane] == std::ptr::null_mut() {
            return None;
        }

        Some(Self {
            next_frame: 0,
            plane,
            offset: if format.is_planar() {
                0
            } else {
                format.get_bytes_per_sample() * channel
            },
            stride: info.get_sample_stride(),
            audio_data: audio_data.clone(),
        })
    }
}

impl<'a, T: AudioFormat> Iterator for SampleIterator<'a, T> {
    type Item = T::SampleType;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_frame >= self.audio_data.frames() as usize {
            return None;
        }

        let sample = unsafe {
            let audio_data = &*self.audio_data.inner;
            let plane_data = audio_data.data[self.plane];
            let sample_ptr: *mut u8 = plane_data.offset((self.offset + self.stride * self.next_frame) as isize);
            let sample_ptr: *mut T::SampleType = sample_ptr as *mut _;

            *sample_ptr
        };

        self.next_frame += 1;

        Some(sample)
    }
}

impl<'a, T: AudioFormat> ExactSizeIterator for SampleIterator<'a, T> {
    fn len(&self) -> usize {
        self.audio_data.frames() as usize
    }
}

impl<'a, T: AudioFormat> AudioData<'a, T> {
    /// For some reason, the reported speaker layout is incorrect and access
    /// to channels out of (real) bounds causes undefined behaviour, such as
    /// crashes.
    pub fn samples(&self, channel: usize)
        -> Option<impl Iterator<Item=T::SampleType> + ExactSizeIterator + 'a> {
        if channel < self.info.speaker_layout().get_channel_count() {
            SampleIterator::new(self, channel)
        } else {
            None
        }
    }

    pub fn samples_normalized(&self, channel: usize)
        -> Option<impl Iterator<Item=f32> + ExactSizeIterator + 'a> {
        self.samples(channel).map(|samples| {
            samples.map(|sample| <T as AudioFormat>::normalize_sample(sample))
        })
    }
}

/// A shared reference to audio data.
/// This type can be in two forms; `AudioData<()>` and `AudioData<T> where T: AudioFormat`.
pub struct AudioData<'a, T> {
    inner: *const audio_data,
    info: &'a AudioOutputInfo,
    __marker: std::marker::PhantomData<T>,
}

impl<'a, T> Clone for AudioData<'a, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            info: self.info,
            __marker: Default::default(),
        }
    }
}

impl<'a, T> AudioData<'a, T> {
    pub fn info(&self) -> &AudioOutputInfo {
        &self.info
    }

    pub fn sample_bytes(&self, channel: usize) -> &[u8] {
        let len = self.info.format().get_bytes_per_sample() * self.frames() as usize;

        unsafe {
            let inner = &*self.inner;

            std::slice::from_raw_parts(inner.data[channel], len)
        }
    }

    pub fn channels(&self) -> impl Iterator<Item=usize> {
        (0..(self.info.speaker_layout().get_channel_count())).into_iter()
    }

    pub fn frames(&self) -> u32 {
        unsafe {
            let inner = &*self.inner;

            inner.frames
        }
    }

    pub fn timestamp(&self) -> u64 {
        unsafe {
            let inner = &*self.inner;

            inner.timestamp
        }
    }

    pub fn upcast(self) -> AudioData<'a, ()> {
        AudioData {
            inner: self.inner,
            info: self.info,
            __marker: Default::default(),
        }
    }
}

impl<'a> AudioData<'a, ()> {
    pub unsafe fn from_raw(inner: *const audio_data, info: &'a AudioOutputInfo) -> Self {
        Self {
            inner,
            info,
            __marker: Default::default(),
        }
    }

    pub fn downcast<T: AudioFormat>(self) -> Option<AudioData<'a, T>> {
        let info = Audio::get().get_output_info();

        if info.format() == T::KIND {
            Some(AudioData {
                inner: self.inner,
                info: self.info,
                __marker: Default::default(),
            })
        } else {
            None
        }
    }

    pub fn samples_normalized(&self, channel: usize) -> Option<Box<dyn IteratorExactSizeIterator<f32> + 'a>> {
        use AudioFormatKind::*;

        macro_rules! match_arm {
            ($audio_format_ty:ty) => {
                paste::expr! {
                    self.clone().downcast::<[< AudioFormat $audio_format_ty >]>()
                        .unwrap().samples_normalized(channel)
                        .map(|iterator| Box::new(iterator) as Box<dyn IteratorExactSizeIterator<f32> + 'a>)
                }
            }
        }

        match self.info.format() {
            InterleavedU8 => match_arm!(InterleavedU8),
            InterleavedI16 => match_arm!(InterleavedI16),
            InterleavedI32 => match_arm!(InterleavedI32),
            InterleavedF32 => match_arm!(InterleavedF32),
            PlanarU8 => match_arm!(PlanarU8),
            PlanarI16 => match_arm!(PlanarI16),
            PlanarI32 => match_arm!(PlanarI32),
            PlanarF32 => match_arm!(PlanarF32),
            Unknown => None,
        }
    }
}

macro_rules! define_audio_format_types {
    {
        $(
            $binding:ident, $name:ident, $interleaved:expr, $sample_type:ty, { $($convert:tt)* }
        );*$(;)?
    } => {
        pub trait AudioFormat: 'static {
            type SampleType: Copy;
            const KIND: AudioFormatKind;

            /// Converts the sample to a normalized range 
            fn normalize_sample(sample: Self::SampleType) -> f32;
        }

        $(
            paste::item! {
                pub struct [< AudioFormat $name >];

                impl AudioFormat for [< AudioFormat $name >] {
                    type SampleType = $sample_type;
                    const KIND: AudioFormatKind = AudioFormatKind::$name;

                    #[inline(always)]
                    fn normalize_sample(sample: Self::SampleType) -> f32 {
                        ($($convert)*)(sample)
                    }
                }
            }
        )*

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum AudioFormatKind {
            Unknown,
            $(
                $name
            ),*
        }

        impl AudioFormatKind {
            pub fn is_planar(self) -> bool {
                use AudioFormatKind::*;

                match self {
                    $(
                        $name => $interleaved,
                    )*
                    _ => false,
                }
            }

            pub fn get_bytes_per_sample(self) -> usize {
                use AudioFormatKind::*;

                match self {
                    Unknown => 0,
                    $(
                        $name => std::mem::size_of::<$sample_type>(),
                    )*
                }
            }

            pub fn from_raw(raw: audio_format) -> Self {
                use AudioFormatKind::*;

                #[allow(non_upper_case_globals)]
                match raw {
                    audio_format_AUDIO_FORMAT_UNKNOWN => Unknown,
                    $(
                        $binding => $name,
                    )*
                    _ => Unknown,
                }
            }

            pub fn into_raw(self) -> audio_format {
                use AudioFormatKind::*;

                match self {
                    Unknown => audio_format_AUDIO_FORMAT_UNKNOWN,
                    $(
                        $name => $binding,
                    )*
                }
            }
        }
    }
}

// TODO: Check these sample conversions. There might be off-by-one errors.
define_audio_format_types! {
    audio_format_AUDIO_FORMAT_U8BIT,        InterleavedU8,  false, u8,  { |sample| (sample as i16 - (std::u8::MAX / 2) as i16) as f32 / (std::u8::MAX / 2) as f32 };
    audio_format_AUDIO_FORMAT_16BIT,        InterleavedI16, false, i16, { |sample| (sample as f32 / std::i16::MAX as f32) };
    audio_format_AUDIO_FORMAT_32BIT,        InterleavedI32, false, i32, { |sample| (sample as f64 / std::i32::MAX as f64) as f32 };
    audio_format_AUDIO_FORMAT_FLOAT,        InterleavedF32, false, f32, { |sample| sample };
    audio_format_AUDIO_FORMAT_U8BIT_PLANAR, PlanarU8,       true,  u8,  { |sample| (sample as i16 - (std::u8::MAX / 2) as i16) as f32 / (std::u8::MAX / 2) as f32 };
    audio_format_AUDIO_FORMAT_16BIT_PLANAR, PlanarI16,      true,  i16, { |sample| (sample as f32 / std::i16::MAX as f32) };
    audio_format_AUDIO_FORMAT_32BIT_PLANAR, PlanarI32,      true,  i32, { |sample| (sample as f64 / std::i32::MAX as f64) as f32 };
    audio_format_AUDIO_FORMAT_FLOAT_PLANAR, PlanarF32,      true,  f32, { |sample| sample };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpeakerLayoutKind {
    Unknown,
    Mono,
    Stereo,
    Surround2Point1,
    Surround4Point0,
    Surround4Point1,
    Surround5Point1,
    Surround7Point1,
}

impl SpeakerLayoutKind {
    pub fn get_channel_count(self) -> usize {
        use SpeakerLayoutKind::*;

        match self {
            Unknown => 0,
            Mono => 1,
            Stereo => 2,
            Surround2Point1 => 3,
            Surround4Point0 => 4,
            Surround4Point1 => 5,
            Surround5Point1 => 6,
            Surround7Point1 => 8,
        }
    }

    pub fn from_raw(raw: speaker_layout) -> Self {
        use SpeakerLayoutKind::*;

        #[allow(non_upper_case_globals)]
        match raw {
            speaker_layout_SPEAKERS_UNKNOWN => Unknown,
            speaker_layout_SPEAKERS_MONO    => Mono,
            speaker_layout_SPEAKERS_STEREO  => Stereo,
            speaker_layout_SPEAKERS_2POINT1 => Surround2Point1,
            speaker_layout_SPEAKERS_4POINT0 => Surround4Point0,
            speaker_layout_SPEAKERS_4POINT1 => Surround4Point1,
            speaker_layout_SPEAKERS_5POINT1 => Surround5Point1,
            speaker_layout_SPEAKERS_7POINT1 => Surround7Point1,
            _ => Unknown,
        }
    }

    pub fn into_raw(self) -> speaker_layout {
        use SpeakerLayoutKind::*;

        match self {
            Unknown         => speaker_layout_SPEAKERS_UNKNOWN,
            Mono            => speaker_layout_SPEAKERS_MONO,
            Stereo          => speaker_layout_SPEAKERS_STEREO,
            Surround2Point1 => speaker_layout_SPEAKERS_2POINT1,
            Surround4Point0 => speaker_layout_SPEAKERS_4POINT0,
            Surround4Point1 => speaker_layout_SPEAKERS_4POINT1,
            Surround5Point1 => speaker_layout_SPEAKERS_5POINT1,
            Surround7Point1 => speaker_layout_SPEAKERS_7POINT1,
        }
    }
}

pub struct AudioOutputInfo {
    inner: *const audio_output_info,
}

impl AudioOutputInfo {
    pub fn name(&self) -> &CStr {
        unsafe {
            let inner = &*self.inner;

            CStr::from_ptr(inner.name)
        }
    }

    pub fn samples_per_sec(&self) -> u32 {
        unsafe {
            let inner = &*self.inner;

            inner.samples_per_sec
        }
    }

    pub fn format(&self) -> AudioFormatKind {
        unsafe {
            let inner = &*self.inner;

            AudioFormatKind::from_raw(inner.format)
        }
    }

    pub fn speaker_layout(&self) -> SpeakerLayoutKind {
        unsafe {
            let inner = &*self.inner;

            SpeakerLayoutKind::from_raw(inner.format)
        }
    }

    /// The number of planes in a block
    pub fn get_planes(&self) -> usize {
        if self.format().is_planar() {
            self.speaker_layout().get_channel_count()
        } else {
            1
        }
    }

    /// The stride of the samples of a channel in a block
    pub fn get_sample_stride(&self) -> usize {
        let format = self.format();

        (
            if format.is_planar() {
                1
            } else {
                self.speaker_layout().get_channel_count()
            }
        ) * format.get_bytes_per_sample()
    }
}

pub type AudioOutputCallback = Box<dyn Fn(AudioData<()>)>;

pub struct Audio {
    inner: *mut audio_t,
}

impl Audio {
    pub fn get() -> Audio {
        Self {
            inner: unsafe { obs_get_audio() },
        }
    }

    pub fn connect_output(&self, mix_index: usize, callback: AudioOutputCallback) -> AudioOutput {
        let callback_ptr = Box::into_raw(Box::new(callback));

        unsafe {
            audio_output_connect(
                self.inner,
                mix_index as size_t, // Mix index to get the raw audio from
                std::ptr::null(), // Conversion information of type `audio_convert_info*` or NULL for no conversion
                Some(global_audio_output_callback),
                callback_ptr as *mut _,
            );
        }

        AudioOutput {
            mix_index,
            callback_ptr,
        }
    }

    pub fn get_output_info(&self) -> AudioOutputInfo {
        unsafe {
            AudioOutputInfo {
                inner: audio_output_get_info(self.inner)
            }
        }
    }

    pub fn is_output_active(&self) -> bool {
        unsafe {
            audio_output_active(self.inner)
        }
    }

    pub fn get_output_block_size(&self) -> usize {
        unsafe {
            audio_output_get_block_size(self.inner) as usize
        }
    }

    pub fn get_output_planes(&self) -> usize {
        unsafe {
            audio_output_get_planes(self.inner) as usize
        }
    }

    pub fn get_output_channels(&self) -> usize {
        unsafe {
            audio_output_get_channels(self.inner) as usize
        }
    }

    pub fn get_output_sample_rate(&self) -> u32 {
        unsafe {
            audio_output_get_sample_rate(self.inner) as u32
        }
    }
}

unsafe extern "C" fn global_audio_output_callback(
    param: *mut ::std::os::raw::c_void,
    _mix_idx: size_t,
    data: *mut audio_data,
) {
    let callback: Box<AudioOutputCallback> = Box::from_raw(param as *mut _);
    let audio_info = Audio::get().get_output_info();
    let data = AudioData::from_raw(data, &audio_info);

    (callback)(data);

    std::mem::forget(callback);
}
