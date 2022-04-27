use sdl2::audio;
use sdl2::audio::AudioCallback;
use sdl2::audio::AudioSpecDesired;
use sdl2::audio::AudioStatus;
use sdl2::Sdl;

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = match self.phase {
                0.0..=0.5 => self.volume,
                _ => -self.volume,
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

pub struct AudioDevice {
    device: audio::AudioDevice<SquareWave>,
}

impl AudioDevice {
    pub fn new(sdl_context: &Sdl) -> AudioDevice {
        let audio_subsystem = sdl_context.audio().unwrap();
        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: None,
        };
        let device = audio_subsystem
            .open_playback(None, &desired_spec, |spec| SquareWave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.25,
            })
            .unwrap();

        let audio_device = AudioDevice { device };
        audio_device
    }

    pub fn set_beep(&self, on: bool) {
        let status = self.device.status();
        if (status == AudioStatus::Paused || status == AudioStatus::Stopped) && on {
            self.device.resume();
        } else if status == AudioStatus::Playing && !on {
            self.device.pause();
        }
    }
}
