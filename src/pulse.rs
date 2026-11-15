//! What the music does to the flower, worked out once per frame on the CPU
//! because a shader keeps nothing between frames.
//!
//! Each ring of petals follows one band of the spectrum: the outer rings
//! the bass, the inner ones the treble. A sudden rise of the bass is a
//! kick, which pushes the petals out in a wave from the centre.

pub const RINGS: usize = 9;
/// The bytes the shaders get: a level per ring, the kick's age and its strength, one spare float.
pub const DATA_BYTES: usize = (RINGS + 3) * 4;

const WAVE_LEN: usize = 1024;
const SPECTRUM_LEN: usize = 512;
/// time, dt, width and height come before the wave.
const HEAD: usize = 4;
const INPUT_BYTES: usize = (HEAD + WAVE_LEN + SPECTRUM_LEN) * 4;

/// The highest bin with any music in it; above it is mostly silence.
const TOP_BIN: f32 = 400.0;
/// Bins 1 to 4: the kick drum and the bass.
const BASS_BINS: std::ops::Range<usize> = 1..5;

const ATTACK_S: f32 = 0.03;
const RELEASE_S: f32 = 0.25;
/// How long a loud passage keeps quieter ones from filling the petals.
const PEAK_FALL_S: f32 = 4.0;
/// Below this a band is not made louder, so hiss does not swell the petals.
const QUIETEST_PEAK: f32 = 0.2;
const BASS_AVERAGE_S: f32 = 0.5;
/// A kick is the bass this much above its recent average.
const KICK_RATIO: f32 = 1.35;
const KICK_MARGIN: f32 = 0.03;
/// No new kick sooner than this after the last one.
const KICK_GAP_S: f32 = 0.2;
/// A kick that never came, so silence draws no wave.
const LONG_AGO_S: f32 = 1000.0;

/// The part of the frame's numbers the pulse needs, read once from the bytes.
pub struct Input<'a> {
    pub dt: f32,
    pub spectrum: &'a [u8],
}

impl<'a> Input<'a> {
    /// The player's frame bytes; anything of another length is refused.
    pub fn parse(bytes: &'a [u8]) -> Option<Input<'a>> {
        if bytes.len() != INPUT_BYTES {
            return None;
        }
        let dt = f32::from_le_bytes(bytes[4..8].try_into().ok()?);
        Some(Input { dt: dt.clamp(0.0, 0.25), spectrum: &bytes[(HEAD + WAVE_LEN) * 4..] })
    }

    fn bin(&self, i: usize) -> f32 {
        f32::from_le_bytes(self.spectrum[i * 4..i * 4 + 4].try_into().unwrap())
    }

    /// The loudest bin of a band. Bands widen with pitch, as hearing does.
    fn band(&self, band: usize) -> f32 {
        let first = TOP_BIN.powf(band as f32 / RINGS as f32) as usize;
        let last = (TOP_BIN.powf((band + 1) as f32 / RINGS as f32) as usize).max(first + 1);
        (first..last).map(|i| self.bin(i)).fold(0.0, f32::max)
    }

    fn bass(&self) -> f32 {
        BASS_BINS.map(|i| self.bin(i)).sum::<f32>() / BASS_BINS.len() as f32
    }
}

pub struct Pulse {
    level: [f32; RINGS],
    peak: [f32; RINGS],
    bass_average: f32,
    kick_age: f32,
    kick: f32,
}

impl Default for Pulse {
    fn default() -> Pulse {
        Pulse { level: [0.0; RINGS], peak: [0.0; RINGS], bass_average: 0.0, kick_age: LONG_AGO_S, kick: 0.0 }
    }
}

impl Pulse {
    /// Moves on by one frame and returns the shaders' data.
    pub fn step(&mut self, input: &Input) -> [u8; DATA_BYTES] {
        let dt = input.dt;
        for ring in 0..RINGS {
            // Ring 0 is the innermost and follows the highest band.
            let now = input.band(RINGS - 1 - ring);
            let tau = if now > self.level[ring] { ATTACK_S } else { RELEASE_S };
            self.level[ring] += (now - self.level[ring]) * (1.0 - (-dt / tau).exp());
            self.peak[ring] = now.max(self.peak[ring] * (-dt / PEAK_FALL_S).exp());
        }
        self.listen_for_a_kick(input.bass(), dt);
        self.data()
    }

    fn listen_for_a_kick(&mut self, bass: f32, dt: f32) {
        self.kick_age = (self.kick_age + dt).min(LONG_AGO_S);
        let threshold = self.bass_average * KICK_RATIO + KICK_MARGIN;
        if bass > threshold && self.kick_age > KICK_GAP_S {
            self.kick_age = 0.0;
            self.kick = ((bass - self.bass_average) / bass.max(f32::EPSILON)).clamp(0.3, 1.0);
        }
        self.bass_average += (bass - self.bass_average) * (1.0 - (-dt / BASS_AVERAGE_S).exp());
    }

    fn data(&self) -> [u8; DATA_BYTES] {
        let rings = (0..RINGS).map(|r| self.level[r] / self.peak[r].max(QUIETEST_PEAK));
        let floats = rings.chain([self.kick_age, self.kick, 0.0]);
        let mut bytes = [0; DATA_BYTES];
        for (chunk, value) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(floats) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn frame(spectrum: impl Fn(usize) -> f32) -> Vec<u8> {
        let head = [0.0, DT, 640.0, 480.0];
        let floats = head.into_iter().chain([0.0; WAVE_LEN]).chain((0..SPECTRUM_LEN).map(spectrum));
        floats.flat_map(f32::to_le_bytes).collect()
    }

    fn run(pulse: &mut Pulse, bytes: &[u8], frames: usize) -> Vec<f32> {
        let input = Input::parse(bytes).unwrap();
        let mut data = [0; DATA_BYTES];
        for _ in 0..frames {
            data = pulse.step(&input);
        }
        data.as_chunks::<4>().0.iter().map(|&c| f32::from_le_bytes(c)).collect()
    }

    const KICK_AGE: usize = RINGS;
    const OUTER: usize = RINGS - 1;

    #[test]
    fn refuses_a_frame_of_another_length() {
        assert!(Input::parse(&[0; 16]).is_none());
    }

    #[test]
    fn silence_leaves_the_flower_still() {
        let data = run(&mut Pulse::default(), &frame(|_| 0.0), 120);
        assert!(data[..RINGS].iter().all(|&l| l == 0.0), "{data:?}");
        assert_eq!(data[KICK_AGE], LONG_AGO_S);
    }

    #[test]
    fn bass_swells_the_outer_ring_and_treble_the_inner() {
        let bass = run(&mut Pulse::default(), &frame(|bin| if bin < 3 { 0.8 } else { 0.0 }), 30);
        assert!(bass[OUTER] > 0.9 && bass[0] == 0.0, "{bass:?}");
        let treble = run(&mut Pulse::default(), &frame(|bin| if bin > 300 { 0.8 } else { 0.0 }), 30);
        assert!(treble[0] > 0.9 && treble[OUTER] == 0.0, "{treble:?}");
    }

    #[test]
    fn quiet_music_still_fills_the_petals() {
        let data = run(&mut Pulse::default(), &frame(|_| 0.3), 30);
        assert!(data[..RINGS].iter().all(|&l| l > 0.9), "{data:?}");
    }

    #[test]
    fn petals_fall_back_when_the_sound_stops() {
        let mut pulse = Pulse::default();
        let loud = run(&mut pulse, &frame(|_| 0.8), 30);
        let after = run(&mut pulse, &frame(|_| 0.0), 30);
        assert!(after[0] < loud[0] * 0.25, "{} then {}", loud[0], after[0]);
    }

    #[test]
    fn a_sudden_bass_is_a_kick_and_a_steady_one_is_not() {
        let mut pulse = Pulse::default();
        run(&mut pulse, &frame(|_| 0.1), 60);
        let hit = run(&mut pulse, &frame(|_| 0.7), 1);
        assert_eq!(hit[KICK_AGE], 0.0);
        let held = run(&mut pulse, &frame(|_| 0.7), 120);
        assert!(held[KICK_AGE] > 1.0, "a held bass kicked again: {held:?}");
    }

    #[test]
    fn kicks_come_no_closer_than_the_gap() {
        let mut pulse = Pulse::default();
        run(&mut pulse, &frame(|_| 0.0), 10);
        run(&mut pulse, &frame(|_| 0.7), 1);
        run(&mut pulse, &frame(|_| 0.0), 2);
        let again = run(&mut pulse, &frame(|_| 0.9), 1);
        assert!(again[KICK_AGE] > 0.0, "kicked again after {} s", 3.0 * DT);
    }
}
