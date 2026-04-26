use crate::fm_synth::SynthEngine;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// System default-output audio probe. Captures `device + config` so the
/// sample rate can be read up front and the same handles reused at stream
/// construction — avoids querying the OS twice at startup.
pub struct AudioProbe {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
}

impl AudioProbe {
    pub fn default_output() -> Self {
        Self::try_default_output().expect("No output device available")
    }

    /// Fallible variant: returns `None` if the host has no default output device
    /// or the device fails to report its config. Used by tests so they can run
    /// in headless environments without panicking.
    pub fn try_default_output() -> Option<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        Some(Self { device, config })
    }

    pub fn sample_rate(&self) -> f32 {
        self.config.sample_rate() as f32
    }
}

pub struct AudioEngine {
    _stream: cpal::Stream,
    _underrun_counter: Arc<AtomicUsize>,
}

impl AudioEngine {
    pub fn new(
        probe: AudioProbe,
        engine: Arc<Mutex<SynthEngine>>,
        underrun_counter: Arc<AtomicUsize>,
    ) -> Self {
        let AudioProbe { device, config } = probe;
        let sample_rate = config.sample_rate();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                Self::build_stream::<f32>(&device, &config.into(), engine, underrun_counter.clone())
            }
            cpal::SampleFormat::I16 => {
                Self::build_stream::<i16>(&device, &config.into(), engine, underrun_counter.clone())
            }
            cpal::SampleFormat::U16 => {
                Self::build_stream::<u16>(&device, &config.into(), engine, underrun_counter.clone())
            }
            format => panic!("Unsupported sample format: {:?}", format),
        };

        stream.play().expect("Failed to start audio stream");

        log::info!(
            "Audio engine initialized with {} Hz sample rate",
            sample_rate
        );

        Self {
            _stream: stream,
            _underrun_counter: underrun_counter,
        }
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        engine: Arc<Mutex<SynthEngine>>,
        underrun_counter: Arc<AtomicUsize>,
    ) -> cpal::Stream
    where
        T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
    {
        let channels = config.channels as usize;
        let mut samples_since_snapshot = 0u32;
        let snapshot_interval = 1024; // Update snapshot every N samples

        device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    match engine.try_lock() {
                        Ok(mut synth) => {
                            // Process commands at the start of each buffer
                            synth.process_commands();

                            for frame in data.chunks_mut(channels) {
                                let (left, right) = synth.process_stereo();

                                if channels >= 2 {
                                    frame[0] = T::from_sample(left);
                                    frame[1] = T::from_sample(right);
                                } else {
                                    frame[0] = T::from_sample((left + right) * 0.5);
                                }

                                samples_since_snapshot += 1;
                            }

                            // Update snapshot periodically (not every sample)
                            if samples_since_snapshot >= snapshot_interval {
                                synth.update_snapshot();
                                samples_since_snapshot = 0;
                            }
                        }
                        Err(_) => {
                            let underrun_count = underrun_counter.fetch_add(1, Ordering::Relaxed);
                            if underrun_count.is_multiple_of(500) {
                                log::warn!(
                                    "AUDIO WARNING: {} buffer underruns detected",
                                    underrun_count
                                );
                            }

                            for frame in data.chunks_mut(channels) {
                                let value = T::from_sample(0.0);
                                for channel_sample in frame.iter_mut() {
                                    *channel_sample = value;
                                }
                            }
                        }
                    }
                },
                |err| log::error!("Audio stream error: {}", err),
                None,
            )
            .expect("Failed to build output stream")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm_synth::create_synth;

    #[test]
    fn try_default_output_returns_a_valid_sample_rate_when_available() {
        let Some(probe) = AudioProbe::try_default_output() else {
            return; // headless host: no output device
        };
        let sr = probe.sample_rate();
        assert!((8_000.0..=384_000.0).contains(&sr), "implausible sample rate: {sr}");
    }

    #[test]
    fn audio_engine_new_runs_when_a_device_is_available() {
        let Some(probe) = AudioProbe::try_default_output() else {
            return;
        };
        let sr = probe.sample_rate();
        let (engine, _ctrl) = create_synth(sr);
        let engine = Arc::new(Mutex::new(engine));
        let underrun = Arc::new(AtomicUsize::new(0));
        let _audio = AudioEngine::new(probe, engine, underrun.clone());
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert_eq!(underrun.load(Ordering::Relaxed), 0);
    }
}
