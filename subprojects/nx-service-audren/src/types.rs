//! Audio renderer wire-layout types.

use static_assertions::const_assert_eq;

/// Audio renderer revision 1 ("REV1"). \[1.0.0+\]
pub const REVISION_1: u32 = 0x3156_4552;

/// Audio renderer revision 2 ("REV2"). \[2.0.0+\]
pub const REVISION_2: u32 = 0x3256_4552;

/// Audio renderer revision 3 ("REV3"). \[3.0.0+\]
pub const REVISION_3: u32 = 0x3356_4552;

/// Audio renderer revision 4 ("REV4"). \[4.0.0+\]
pub const REVISION_4: u32 = 0x3456_4552;

/// Audio renderer revision 5 ("REV5"). \[6.0.0+\]
pub const REVISION_5: u32 = 0x3556_4552;

/// Audio renderer revision 6 ("REV6"). \[6.1.0+\]
pub const REVISION_6: u32 = 0x3656_4552;

/// Timer frequency in Hz.
pub const TIMER_FREQ_HZ: f32 = 200.0;

/// Timer period in milliseconds.
pub const TIMER_PERIOD_MS: f32 = 5.0;

/// Samples per frame at 32 kHz output rate.
pub const SAMPLES_PER_FRAME_32KHZ: u32 = 160;

/// Samples per frame at 48 kHz output rate.
pub const SAMPLES_PER_FRAME_48KHZ: u32 = 240;

/// Required alignment for the input parameter buffer.
pub const INPUT_PARAM_ALIGNMENT: usize = 0x1000;

/// Required alignment for the output parameter buffer.
pub const OUTPUT_PARAM_ALIGNMENT: usize = 0x10;

/// Required alignment for memory pool buffers.
pub const MEMPOOL_ALIGNMENT: usize = 0x1000;

/// Required alignment for audio data buffers.
pub const BUFFER_ALIGNMENT: usize = 0x40;

/// The final mix node ID (always 0).
pub const FINAL_MIX_ID: u32 = 0;

/// Sentinel for an unused mix ID.
pub const UNUSED_MIX_ID: u32 = 0x7FFF_FFFF;

/// Sentinel for an unused splitter ID.
pub const UNUSED_SPLITTER_ID: u32 = 0xFFFF_FFFF;

/// Audio renderer output sample rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OutputRate {
    /// 32 kHz output.
    Rate32kHz = 0,
    /// 48 kHz output.
    Rate48kHz = 1,
}

/// Memory pool state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MemPoolState {
    Invalid = 0,
    New = 1,
    RequestDetach = 2,
    Detached = 3,
    RequestAttach = 4,
    Attached = 5,
    Released = 6,
}

/// Voice play state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VoicePlayState {
    Started = 0,
    Stopped = 1,
    Paused = 2,
}

/// PCM sample format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcmFormat {
    Invalid = 0,
    Int8 = 1,
    Int16 = 2,
    Int24 = 3,
    Int32 = 4,
    Float = 5,
    Adpcm = 6,
}

/// Sink type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SinkType {
    Invalid = 0,
    Device = 1,
    CircularBuffer = 2,
}

/// Wire-layout parameter struct sent to `OpenAudioRenderer` and
/// `GetWorkBufferSize`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct AudioRendererParameter {
    pub sample_rate: i32,
    pub sample_count: i32,
    pub mix_buffer_count: i32,
    pub submix_count: i32,
    pub voice_count: i32,
    pub sink_count: i32,
    pub effect_count: i32,
    pub _unk1: i32,
    pub _unk2: u8,
    pub _pad1: [u8; 3],
    pub splitter_count: i32,
    pub _unk3: i32,
    pub _unk4: i32,
    pub revision: u32,
}

const_assert_eq!(size_of::<AudioRendererParameter>(), 0x34);

/// Wire-layout input for `OpenAudioRenderer` (cmd 0):
/// `{ AudioRendererParameter, u32 pad, u64 work_buffer_size, u64 aruid }`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct OpenAudioRendererIn {
    pub param: AudioRendererParameter,
    pub _pad: u32,
    pub work_buffer_size: u64,
    pub aruid: u64,
}

const_assert_eq!(size_of::<OpenAudioRendererIn>(), 0x48);

/// Header for both input and output update data buffers.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct UpdateDataHeader {
    pub revision: u32,
    pub behavior_sz: u32,
    pub mempools_sz: u32,
    pub voices_sz: u32,
    pub channels_sz: u32,
    pub effects_sz: u32,
    pub mixes_sz: u32,
    pub sinks_sz: u32,
    pub perfmgr_sz: u32,
    pub _pad: [u32; 6],
    pub total_sz: u32,
}

const_assert_eq!(size_of::<UpdateDataHeader>(), 0x40);

/// Behavior info (input direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BehaviorInfoIn {
    pub revision: u32,
    pub _pad: u32,
    pub flags: u64,
}

const_assert_eq!(size_of::<BehaviorInfoIn>(), 0x10);

/// Behavior info (output direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BehaviorInfoOut {
    pub _unknown: [u64; 20],
    pub _pad: [u64; 2],
}

const_assert_eq!(size_of::<BehaviorInfoOut>(), 0xB0);

/// Memory pool info (input direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct MemPoolInfoIn {
    pub address: u64,
    pub size: u64,
    pub state: MemPoolState,
    pub _pad: [u32; 3],
}

const_assert_eq!(size_of::<MemPoolInfoIn>(), 0x20);

/// Memory pool info (output direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct MemPoolInfoOut {
    pub new_state: MemPoolState,
    pub _pad: [u32; 3],
}

const_assert_eq!(size_of::<MemPoolInfoOut>(), 0x10);

/// Channel info (input direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ChannelInfoIn {
    pub id: u32,
    pub mix: [f32; 24],
    pub is_used: u8,
    pub _pad: [u8; 11],
}

const_assert_eq!(size_of::<ChannelInfoIn>(), 0x70);

/// Biquad filter parameters for a voice.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BiquadFilter {
    pub enable: u8,
    pub _pad: u8,
    pub numerator: [i16; 3],
    pub denominator: [i16; 2],
}

const_assert_eq!(size_of::<BiquadFilter>(), 0x0C);

/// ADPCM codec parameters (16 coefficients).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct AdpcmParameters {
    pub coefficients: [u16; 16],
}

const_assert_eq!(size_of::<AdpcmParameters>(), 0x20);

/// ADPCM codec context.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct AdpcmContext {
    pub index: u16,
    pub history0: i16,
    pub history1: i16,
}

const_assert_eq!(size_of::<AdpcmContext>(), 0x06);

/// Wave buffer descriptor for a voice.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct WaveBuf {
    pub address: u64,
    pub size: u64,
    pub start_sample_offset: i32,
    pub end_sample_offset: i32,
    pub is_looping: u8,
    pub end_of_stream: u8,
    pub sent_to_server: u8,
    pub _pad1: [u8; 5],
    pub context_addr: u64,
    pub context_sz: u64,
    pub _pad2: u64,
}

const_assert_eq!(size_of::<WaveBuf>(), 0x38);

/// Voice info (input direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct VoiceInfoIn {
    pub id: u32,
    pub node_id: u32,
    pub is_new: u8,
    pub is_used: u8,
    pub state: u8,
    pub sample_format: u8,
    pub sample_rate: u32,
    pub priority: u32,
    pub sorting_order: u32,
    pub channel_count: u32,
    pub pitch: f32,
    pub volume: f32,
    pub biquads: [BiquadFilter; 2],
    pub wavebuf_count: u32,
    pub wavebuf_head: i16,
    pub _pad1: u16,
    pub _pad2: u32,
    pub extra_params_ptr: u64,
    pub extra_params_sz: u64,
    pub dest_mix_id: u32,
    pub dest_splitter_id: u32,
    pub wavebufs: [WaveBuf; 4],
    pub channel_ids: [u32; 6],
    pub _pad3: [u8; 24],
}

const_assert_eq!(size_of::<VoiceInfoIn>(), 0x170);

/// Voice info (output direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct VoiceInfoOut {
    pub played_sample_count: u64,
    pub num_wavebufs_consumed: u32,
    pub voice_drops_count: u32,
}

const_assert_eq!(size_of::<VoiceInfoOut>(), 0x10);

/// Mix info (input direction).
///
/// Contains a 24×24 float mixing matrix (`mix[src_index][dest_index]`).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct MixInfoIn {
    pub volume: f32,
    pub sample_rate: u32,
    pub buffer_count: u32,
    pub is_used: u8,
    pub _pad1: [u8; 3],
    pub mix_id: u32,
    pub _pad2: u32,
    pub node_id: u32,
    pub _pad3: [u32; 2],
    pub mix: [[f32; 24]; 24],
    pub dest_mix_id: u32,
    pub dest_splitter_id: u32,
    pub _pad4: u32,
}

const_assert_eq!(size_of::<MixInfoIn>(), 0x930);

/// Down-mix parameter coefficients.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct DownMixParameters {
    pub coefficients: [u8; 16],
}

const_assert_eq!(size_of::<DownMixParameters>(), 0x10);

/// Device sink info (input direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct DeviceSinkInfoIn {
    pub name: [u8; 255],
    pub _pad1: u8,
    pub input_count: u32,
    pub inputs: [u8; 6],
    pub _pad2: u8,
    pub downmix_params_enabled: u8,
    pub downmix_params: DownMixParameters,
}

const_assert_eq!(size_of::<DeviceSinkInfoIn>(), 0x11C);

/// Circular buffer sink info (input direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct CircularBufferSinkInfoIn {
    pub buffer_ptr: u64,
    pub buffer_sz: u32,
    pub input_count: u32,
    pub sample_count: u32,
    pub last_read_offset: u32,
    pub sample_format: u32,
    pub inputs: [u8; 6],
    pub _pad: [u8; 6],
}

const_assert_eq!(size_of::<CircularBufferSinkInfoIn>(), 0x28);

/// Sink info (input direction).
///
/// The payload bytes are interpreted as [`DeviceSinkInfoIn`] or
/// [`CircularBufferSinkInfoIn`] depending on `sink_type`.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct SinkInfoIn {
    pub sink_type: u8,
    pub is_used: u8,
    pub _pad1: [u8; 2],
    pub node_id: u32,
    pub _pad2: [u64; 3],
    pub payload: [u8; 0x11C],
}

const_assert_eq!(size_of::<SinkInfoIn>(), 0x140);

/// Sink info (output direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct SinkInfoOut {
    pub last_written_offset: u32,
    pub _unk1: u32,
    pub _unk2: u64,
    pub _pad: [u64; 2],
}

const_assert_eq!(size_of::<SinkInfoOut>(), 0x20);

/// Performance buffer info (input direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct PerformanceBufferInfoIn {
    pub detail_target: u32,
    pub _pad: [u32; 3],
}

const_assert_eq!(size_of::<PerformanceBufferInfoIn>(), 0x10);

/// Performance buffer info (output direction).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct PerformanceBufferInfoOut {
    pub written_sz: u32,
    pub _pad: [u32; 3],
}

const_assert_eq!(size_of::<PerformanceBufferInfoOut>(), 0x10);
