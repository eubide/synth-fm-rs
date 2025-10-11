use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Lock-free triple buffer for real-time parameter updates
/// GUI writes to one buffer, audio reads from another, third is for swapping
pub struct TripleBuffer<T: Clone> {
    buffers: [T; 3],
    write_index: AtomicUsize,
    read_index: AtomicUsize,
    swap_requested: AtomicBool,
}

impl<T: Clone> TripleBuffer<T> {
    pub fn new(initial_value: T) -> Self {
        Self {
            buffers: [initial_value.clone(), initial_value.clone(), initial_value],
            write_index: AtomicUsize::new(0),
            read_index: AtomicUsize::new(1),
            swap_requested: AtomicBool::new(false),
        }
    }

    /// Non-blocking write for GUI thread
    pub fn write(&mut self, data: T) {
        let write_idx = self.write_index.load(Ordering::Relaxed);
        self.buffers[write_idx] = data;
        self.swap_requested.store(true, Ordering::Release);
    }

    /// Lock-free read for audio thread
    pub fn read(&self) -> &T {
        // Check if GUI requested a swap
        if self
            .swap_requested
            .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            // Swap read and write buffers
            let old_read = self.read_index.load(Ordering::Relaxed);
            let old_write = self.write_index.load(Ordering::Relaxed);

            self.read_index.store(old_write, Ordering::Relaxed);
            self.write_index.store(old_read, Ordering::Relaxed);
        }

        let read_idx = self.read_index.load(Ordering::Relaxed);
        &self.buffers[read_idx]
    }
}

/// Real-time safe synthesizer parameters
#[derive(Debug, Clone)]
pub struct SynthParameters {
    pub algorithm: u8,
    pub master_volume: f32,
    pub pitch_bend: f32,
    pub mod_wheel: f32,
    pub master_tune: f32,
    pub pitch_bend_range: f32,
    pub portamento_enable: bool,
    pub portamento_time: f32,
    pub mono_mode: bool,
}

impl Default for SynthParameters {
    fn default() -> Self {
        Self {
            algorithm: 1,
            master_volume: 0.7,
            pitch_bend: 0.0,
            mod_wheel: 0.0,
            master_tune: 0.0,
            pitch_bend_range: 2.0,
            portamento_enable: false,
            portamento_time: 50.0,
            mono_mode: false,
        }
    }
}

/// Lock-free synthesizer state for real-time audio processing
pub struct LockFreeSynth {
    pub global_params: TripleBuffer<SynthParameters>,

    // Atomic values for simple parameters
    pub sustain_pedal: AtomicBool,
    pub panic_requested: AtomicBool,
}

impl LockFreeSynth {
    pub fn new() -> Self {
        Self {
            global_params: TripleBuffer::new(SynthParameters::default()),
            sustain_pedal: AtomicBool::new(false),
            panic_requested: AtomicBool::new(false),
        }
    }

    /// Update global parameter (GUI thread)
    pub fn set_global_param(&mut self, params: SynthParameters) {
        self.global_params.write(params);
    }

    /// Get current global parameters (audio thread)
    pub fn get_global_params(&self) -> &SynthParameters {
        self.global_params.read()
    }

    /// Request panic (from any thread)
    pub fn request_panic(&self) {
        self.panic_requested.store(true, Ordering::Release);
    }

    /// Check and clear panic request (audio thread)
    pub fn check_panic_request(&self) -> bool {
        self.panic_requested
            .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    /// Set sustain pedal (from any thread)
    pub fn set_sustain_pedal(&self, pressed: bool) {
        self.sustain_pedal.store(pressed, Ordering::Release);
    }

    /// Get sustain pedal state (audio thread)
    pub fn get_sustain_pedal(&self) -> bool {
        self.sustain_pedal.load(Ordering::Acquire)
    }
}

unsafe impl<T: Clone + Send> Send for TripleBuffer<T> {}
unsafe impl<T: Clone + Send> Sync for TripleBuffer<T> {}
