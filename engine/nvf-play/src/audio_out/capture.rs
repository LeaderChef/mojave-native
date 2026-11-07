//! Opt-in unattended mixer capture. This replaces, rather than taps alongside, the
//! hardware consumer. [Design] Frame-paced output verifies decoding and mixing, not
//! CoreAudio latency or what a listener hears. Nothing in this module opens a device.

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use super::*;

const MAX_FRAMES: u64 = RATE as u64 * 120;

/// Prepared before AudioOutPlugin builds, so an invalid path fails without opening
/// a speaker. Creates a new file only; never overwrites an earlier test or save.
#[derive(Resource)]
pub struct AudioCaptureSettings {
    wav: Wav<BufWriter<File>>,
}

impl AudioCaptureSettings {
    pub fn create(path: &Path) -> io::Result<Self> {
        fnv_install::guard_write(path, None).map_err(io::Error::other)?;
        let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        fnv_install::guard_write(parent, None).map_err(io::Error::other)?;
        std::fs::create_dir_all(parent)?;
        let path = parent.canonicalize()?.join(path.file_name().ok_or_else(|| io::Error::other("missing file name"))?);
        fnv_install::guard_write(&path, None).map_err(io::Error::other)?;
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Self { wav: Wav::new(BufWriter::with_capacity(64 * 1024, file), MAX_FRAMES)? })
    }
}

#[derive(Default)]
struct Clock {
    accounted: u64,
    skipped: u64,
}

impl Clock {
    fn due(&mut self, target: u64) -> usize {
        let due = target.saturating_sub(self.accounted);
        let take = due.min(u64::from(RATE)); // Never catch up more than one second per frame.
        self.skipped += due - take;
        self.accounted = self.accounted.max(target);
        take as usize
    }
}

#[derive(Resource)]
pub(super) struct Capture {
    shared: Arc<Mutex<MixerSource>>,
    stats: Arc<Stats>,
    wav: Wav<BufWriter<File>>,
    started: Instant,
    clock: Clock,
    error: Option<String>,
}

impl Capture {
    pub(super) fn new(settings: AudioCaptureSettings, shared: Arc<Mutex<MixerSource>>, stats: Arc<Stats>) -> Self {
        info!("audio capture: silent mixer consumer; no output device; 48000 Hz stereo float WAV, limit 120 s");
        Self { shared, stats, wav: settings.wav, started: Instant::now(), clock: Clock::default(), error: None }
    }

    fn drain(&mut self) {
        let target = (self.started.elapsed().as_nanos() * u128::from(RATE) / 1_000_000_000).min(u128::from(u64::MAX)) as u64;
        let mut samples = self.clock.due(target) * CHANNELS as usize;
        while samples > 0 {
            let n = samples.min(BLOCK);
            let mut buf = [0.0; BLOCK];
            let mut i = 0;
            pull(&self.shared, &self.stats, n, |s| { buf[i] = s; i += 1; });
            // File I/O is outside the mixer lock. Past the limit (or on error) the
            // source still drains so one-shots, voice and music continue to finish.
            if self.error.is_none() {
                if let Err(e) = self.wav.write(&buf[..n]) {
                    self.error = Some(e.to_string());
                    error!("audio capture: {e}; continuing silent discard drain");
                }
            }
            samples -= n;
        }
    }
}

pub(super) fn drain(capture: Option<ResMut<Capture>>) {
    if let Some(mut capture) = capture { capture.drain(); }
}

pub(super) fn finish(mut exit: MessageReader<AppExit>, capture: Option<ResMut<Capture>>) {
    if exit.read().last().is_none() { return; }
    let Some(mut c) = capture else { return; };
    c.drain();
    if let Err(e) = c.wav.finish() { c.error = Some(e.to_string()); }
    let samples = c.wav.samples;
    let rms = (c.wav.squares / samples.max(1) as f64).sqrt();
    info!("audio capture summary: complete={} frames={} peak={:.6} rms={:.6} nonzero={} over_one={} nonfinite={} skipped_frames={} limit_reached={} error={:?}",
        c.error.is_none() && c.wav.finished, samples / u64::from(CHANNELS), c.wav.peak, rms,
        c.wav.nonzero, c.wav.over_one, c.wav.nonfinite, c.clock.skipped, c.wav.limit_reached, c.error);
}

/// Small bounded IEEE-float WAV writer. Values over 1 are preserved for diagnosis;
/// nonfinite values become silence and are counted as a failed-quality test.
struct Wav<W: Write + Seek> {
    output: W,
    max_samples: u64,
    samples: u64,
    squares: f64,
    peak: f32,
    nonzero: u64,
    over_one: u64,
    nonfinite: u64,
    finished: bool,
    limit_reached: bool,
}

impl<W: Write + Seek> Wav<W> {
    fn new(mut output: W, frames: u64) -> io::Result<Self> {
        output.write_all(&[0; 44])?;
        Ok(Self { output, max_samples: frames.min(MAX_FRAMES) * u64::from(CHANNELS), samples: 0,
            squares: 0.0, peak: 0.0, nonzero: 0, over_one: 0, nonfinite: 0, finished: false, limit_reached: false })
    }

    fn write(&mut self, samples: &[f32]) -> io::Result<()> {
        if self.finished { return Ok(()); }
        if !samples.len().is_multiple_of(CHANNELS as usize) { return Err(io::Error::other("partial stereo frame")); }
        let count = (self.max_samples - self.samples).min(samples.len() as u64) as usize;
        for &s in &samples[..count] {
            let value = if s.is_finite() { s } else { self.nonfinite += 1; 0.0 };
            self.output.write_all(&value.to_le_bytes())?;
            self.peak = self.peak.max(value.abs());
            self.squares += f64::from(value).powi(2);
            self.nonzero += u64::from(value != 0.0);
            self.over_one += u64::from(value.abs() > 1.0);
            self.samples += 1;
        }
        if self.samples == self.max_samples {
            self.limit_reached = true;
            self.finish()?;
        }
        Ok(())
    }

    fn finish(&mut self) -> io::Result<()> {
        if self.finished { return Ok(()); }
        let bytes = u32::try_from(self.samples * 4).map_err(io::Error::other)?;
        let mut header = Vec::with_capacity(44);
        header.extend(b"RIFF"); header.extend((bytes + 36).to_le_bytes()); header.extend(b"WAVEfmt ");
        header.extend(16_u32.to_le_bytes()); header.extend(3_u16.to_le_bytes()); // IEEE float
        header.extend(CHANNELS.to_le_bytes()); header.extend(RATE.to_le_bytes());
        header.extend((RATE * u32::from(CHANNELS) * 4).to_le_bytes());
        header.extend((CHANNELS * 4).to_le_bytes()); header.extend(32_u16.to_le_bytes());
        header.extend(b"data"); header.extend(bytes.to_le_bytes());
        self.output.seek(SeekFrom::Start(0))?;
        self.output.write_all(&header)?;
        self.output.flush()?;
        self.finished = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn silent_capture_preserves_channels_and_float_peak_and_stops_at_limit() {
        let mut wav = Wav::new(Cursor::new(Vec::new()), 2).unwrap();
        wav.write(&[0.25, -0.5, 1.5, -1.25, 9.0, 9.0]).unwrap();
        wav.write(&[9.0, 9.0]).unwrap();
        assert!(wav.finished && wav.limit_reached);
        assert_eq!((wav.samples, wav.over_one), (4, 2));
        let data = wav.output.get_ref();
        assert_eq!(data.len(), 44 + 16);
        assert_eq!(&data[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(data[4..8].try_into().unwrap()), 52);
        assert_eq!(u16::from_le_bytes(data[20..22].try_into().unwrap()), 3);
        assert_eq!(u16::from_le_bytes(data[22..24].try_into().unwrap()), 2);
        let values: Vec<f32> = data[44..].as_chunks::<4>().0.iter().copied().map(f32::from_le_bytes).collect();
        assert_eq!(values, [0.25, -0.5, 1.5, -1.25]);
    }

    #[test]
    fn silent_capture_early_finish_and_nonfinite_are_reported() {
        let mut wav = Wav::new(Cursor::new(Vec::new()), MAX_FRAMES + 1).unwrap();
        assert_eq!(wav.max_samples, MAX_FRAMES * 2);
        assert!(wav.write(&[0.0]).is_err());
        wav.write(&[f32::NAN, f32::INFINITY]).unwrap();
        wav.finish().unwrap(); wav.finish().unwrap();
        assert_eq!((wav.samples, wav.nonfinite), (2, 2));
        assert!(wav.finished && !wav.limit_reached);
        assert_eq!(wav.output.get_ref().len(), 52);
    }

    #[test]
    fn silent_capture_clock_keeps_fractional_progress_and_bounds_stalls() {
        let mut clock = Clock::default();
        assert_eq!(clock.due(1), 1);
        assert_eq!(clock.due(1), 0);
        assert_eq!(clock.due(2), 1);
        assert_eq!(clock.due(u64::from(RATE) * 3 + 2), RATE as usize);
        assert_eq!(clock.skipped, u64::from(RATE) * 2);
    }

    #[test]
    fn silent_capture_does_not_overwrite_existing_data() {
        let dir = std::env::temp_dir().join(format!("nvf-silent-capture-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("existing.wav");
        std::fs::write(&path, b"preserved").unwrap();
        assert!(AudioCaptureSettings::create(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"preserved");
        std::fs::remove_file(path).unwrap(); std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn silent_capture_consumes_real_category_gains_and_resampled_sources_once() {
        use rodio::buffer::SamplesBuffer;
        let volumes = SoundVolumes::default();
        volumes.set_master(0.25);
        volumes.set(SoundCategory::Effects, 1.0);
        let format = || rodio::mixer::mixer(NonZero::new(CHANNELS).unwrap(), NonZero::new(RATE).unwrap());
        let (category, category_source) = format();
        let (run_long, run_source) = format();
        run_long.add(Gain::new(category_source, volumes.clone(), SoundCategory::Effects));
        category.add(SamplesBuffer::new(NonZero::new(2).unwrap(), NonZero::new(RATE).unwrap(), [0.1, -0.2].repeat(32)));
        category.add(SamplesBuffer::new(NonZero::new(1).unwrap(), NonZero::new(RATE / 2).unwrap(), vec![0.4; 32]));
        let stats = Stats::default();
        let mut mixed = Vec::new();
        pull(&Mutex::new(run_source), &stats, 32, |s| mixed.push(s));
        for frame in mixed.as_chunks::<2>().0 {
            assert!((frame[0] - 0.125).abs() < 1e-5 && (frame[1] - 0.05).abs() < 1e-5, "{frame:?}");
        }
        assert_eq!(stats.frames.load(Ordering::Relaxed), 16);
        assert_eq!(stats.opens.load(Ordering::Relaxed), 0);
        assert_eq!(volumes.0.audible[SoundCategory::Effects.index()].load(Ordering::Relaxed), 16);
        let mut wav = Wav::new(Cursor::new(Vec::new()), 16).unwrap();
        wav.write(&mixed).unwrap();
        let decoder = rodio::Decoder::new(Cursor::new(wav.output.into_inner())).unwrap();
        let decoded: Vec<f32> = decoder.collect();
        assert_eq!(decoded, mixed, "the final WAV roundtrips the mixed float samples");
    }
}
