use crate::fm_synth::FmSynthesizer;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct AudioEngine {
    _stream: cpal::Stream,
}

impl AudioEngine {
    pub fn new(synthesizer: Arc<Mutex<FmSynthesizer>>) -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No output device available");

        let config = device
            .default_output_config()
            .expect("Failed to get default output config");

        let sample_rate = config.sample_rate().0;

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                Self::build_stream::<f32>(&device, &config.into(), synthesizer)
            }
            cpal::SampleFormat::I16 => {
                Self::build_stream::<i16>(&device, &config.into(), synthesizer)
            }
            cpal::SampleFormat::U16 => {
                Self::build_stream::<u16>(&device, &config.into(), synthesizer)
            }
            _ => panic!("Unsupported sample format"),
        };

        stream.play().expect("Failed to start audio stream");

        println!(
            "Audio engine initialized with {} Hz sample rate",
            sample_rate
        );

        Self { _stream: stream }
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        synthesizer: Arc<Mutex<FmSynthesizer>>,
    ) -> cpal::Stream
    where
        T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
    {
        let channels = config.channels as usize;

        device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    let mut synth = synthesizer.lock().unwrap();

                    for frame in data.chunks_mut(channels) {
                        let sample = synth.process();
                        let value = T::from_sample(sample);

                        for channel_sample in frame.iter_mut() {
                            *channel_sample = value;
                        }
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )
            .expect("Failed to build output stream")
    }
}
