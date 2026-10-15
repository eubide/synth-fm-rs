use crate::fm_synth::SynthEngine;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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

fn device_name(device: &cpal::Device) -> Option<String> {
    device.description().ok().map(|d| d.name().to_string())
}

/// A stream config on `device` at `sample_rate`. The synth's tables are built
/// for one rate, so a device that cannot run at it is not usable.
fn config_at(device: &cpal::Device, sample_rate: u32) -> Option<cpal::SupportedStreamConfig> {
    if let Ok(config) = device.default_output_config() {
        if config.sample_rate() == sample_rate {
            return Some(config);
        }
    }
    device
        .supported_output_configs()
        .ok()?
        .filter(|c| {
            matches!(
                c.sample_format(),
                cpal::SampleFormat::F32 | cpal::SampleFormat::I16 | cpal::SampleFormat::U16
            )
        })
        .find_map(|c| c.try_with_sample_rate(sample_rate))
}

pub struct AudioEngine {
    stream: cpal::Stream,
    device_name: String,
    sample_rate: u32,
    engine: Arc<Mutex<SynthEngine>>,
    underrun_counter: Arc<AtomicUsize>,
    /// Set from the stream's error callback when the device goes away.
    stream_failed: Arc<AtomicBool>,
}

impl AudioEngine {
    pub fn new(
        probe: AudioProbe,
        engine: Arc<Mutex<SynthEngine>>,
        underrun_counter: Arc<AtomicUsize>,
    ) -> Self {
        let AudioProbe { device, config } = probe;
        let sample_rate = config.sample_rate();
        let stream_failed = Arc::new(AtomicBool::new(false));

        let stream = Self::open_stream(
            &device,
            config,
            engine.clone(),
            underrun_counter.clone(),
            stream_failed.clone(),
        )
        .expect("Failed to start audio stream");

        log::info!(
            "Audio engine initialized with {} Hz sample rate",
            sample_rate
        );

        Self {
            stream,
            device_name: device_name(&device).unwrap_or_default(),
            sample_rate,
            engine,
            underrun_counter,
            stream_failed,
        }
    }

    pub fn output_device_names() -> Vec<String> {
        cpal::default_host()
            .output_devices()
            .map(|devices| devices.filter_map(|d| device_name(&d)).collect())
            .unwrap_or_default()
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn switch_to(&mut self, name: &str) -> Result<(), String> {
        let device = cpal::default_host()
            .output_devices()
            .map_err(|e| e.to_string())?
            .find(|d| device_name(d).as_deref() == Some(name))
            .ok_or_else(|| format!("{}: not found", name))?;
        self.reopen(&device, name)
    }

    /// After the device was lost, moves the stream to the system default output.
    /// `None` while the stream is healthy; a failed attempt is retried next call.
    pub fn recover_if_failed(&mut self) -> Option<Result<String, String>> {
        if !self.stream_failed.swap(false, Ordering::Relaxed) {
            return None;
        }
        let result = cpal::default_host()
            .default_output_device()
            .ok_or_else(|| "no output device".to_string())
            .and_then(|device| {
                let name = device_name(&device).unwrap_or_default();
                self.reopen(&device, &name).map(|()| name)
            });
        if result.is_err() {
            self.stream_failed.store(true, Ordering::Relaxed);
        }
        Some(result)
    }

    fn reopen(&mut self, device: &cpal::Device, name: &str) -> Result<(), String> {
        let config = config_at(device, self.sample_rate)
            .ok_or_else(|| format!("{} cannot run at {} Hz", name, self.sample_rate))?;
        // Stop the old stream first: two live callbacks would contend for the engine lock.
        let _ = self.stream.pause();
        let stream = Self::open_stream(
            device,
            config,
            self.engine.clone(),
            self.underrun_counter.clone(),
            self.stream_failed.clone(),
        )
        .inspect_err(|_| {
            let _ = self.stream.play();
        })?;
        self.stream = stream;
        self.device_name = name.to_string();
        log::info!("Audio output: {}", name);
        Ok(())
    }

    fn open_stream(
        device: &cpal::Device,
        config: cpal::SupportedStreamConfig,
        engine: Arc<Mutex<SynthEngine>>,
        underrun_counter: Arc<AtomicUsize>,
        stream_failed: Arc<AtomicBool>,
    ) -> Result<cpal::Stream, String> {
        let format = config.sample_format();
        let config = config.into();
        let stream = match format {
            cpal::SampleFormat::F32 => {
                Self::build_stream::<f32>(device, config, engine, underrun_counter, stream_failed)
            }
            cpal::SampleFormat::I16 => {
                Self::build_stream::<i16>(device, config, engine, underrun_counter, stream_failed)
            }
            cpal::SampleFormat::U16 => {
                Self::build_stream::<u16>(device, config, engine, underrun_counter, stream_failed)
            }
            format => return Err(format!("Unsupported sample format: {:?}", format)),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(stream)
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: cpal::StreamConfig,
        engine: Arc<Mutex<SynthEngine>>,
        underrun_counter: Arc<AtomicUsize>,
        stream_failed: Arc<AtomicBool>,
    ) -> Result<cpal::Stream, cpal::Error>
    where
        T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
    {
        let channels = config.channels as usize;
        let mut samples_since_snapshot = 0u32;
        let snapshot_interval = 1024; // Update snapshot every N samples

        device.build_output_stream(
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
            move |err| {
                log::error!("Audio stream error: {}", err);
                if matches!(
                    err.kind(),
                    cpal::ErrorKind::DeviceNotAvailable | cpal::ErrorKind::StreamInvalidated
                ) {
                    stream_failed.store(true, Ordering::Relaxed);
                }
            },
            None,
        )
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
        assert!(
            (8_000.0..=384_000.0).contains(&sr),
            "implausible sample rate: {sr}"
        );
    }

    fn running_engine() -> Option<AudioEngine> {
        let probe = AudioProbe::try_default_output()?;
        let (engine, _ctrl) = create_synth(probe.sample_rate());
        Some(AudioEngine::new(
            probe,
            Arc::new(Mutex::new(engine)),
            Arc::new(AtomicUsize::new(0)),
        ))
    }

    #[test]
    fn switch_to_a_listed_device_moves_the_stream_there() {
        let Some(mut audio) = running_engine() else {
            return;
        };
        let current = audio.device_name().to_string();
        assert!(AudioEngine::output_device_names().contains(&current));

        audio.switch_to(&current).expect("reopen current device");
        assert_eq!(audio.device_name(), current);
    }

    #[test]
    fn switch_to_an_unknown_device_keeps_the_current_one() {
        let Some(mut audio) = running_engine() else {
            return;
        };
        let current = audio.device_name().to_string();
        assert!(audio.switch_to("no such device").is_err());
        assert_eq!(audio.device_name(), current);
    }

    #[test]
    fn a_failed_stream_is_reopened_on_the_default_device() {
        let Some(mut audio) = running_engine() else {
            return;
        };
        assert!(audio.recover_if_failed().is_none());

        audio.stream_failed.store(true, Ordering::Relaxed);
        let reopened = audio.recover_if_failed().expect("recovery attempted");

        assert_eq!(reopened.as_deref(), Ok(audio.device_name()));
        assert!(audio.recover_if_failed().is_none());
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
