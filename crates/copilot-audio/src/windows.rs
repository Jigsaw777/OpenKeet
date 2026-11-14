use std::time::Instant;

use copilot_core::{Source, SourceKind, Span};
use wasapi::{
    DeviceEnumerator, DeviceState, Direction, SampleType, StreamMode, WasapiError, WaveFormat,
};
use windows::Win32::Media::Audio::{
    AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY, AUDCLNT_BUFFERFLAGS_SILENT,
    AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR, IAudioCaptureClient,
};

use crate::{
    AudioError, AudioEvent, Endpoint, FRAME_MS, SAMPLE_RATE, SAMPLES, Stop, elapsed_ms,
    pipeline::Producer,
};

struct Apartment;

impl Apartment {
    fn enter() -> Result<Self, AudioError> {
        wasapi::initialize_mta()
            .ok()
            .map_err(|_| AudioError::new("Could not initialize Windows audio on this thread."))?;
        Ok(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        wasapi::deinitialize();
    }
}

fn unavailable(error: impl std::fmt::Display) -> AudioError {
    AudioError::device(format!(
        "Windows audio is unavailable ({error}). Refresh devices and restart capture."
    ))
}

pub(crate) fn endpoints() -> Result<Vec<Endpoint>, AudioError> {
    let _apartment = Apartment::enter()?;
    let enumerator = DeviceEnumerator::new().map_err(unavailable)?;
    let mut endpoints = Vec::new();
    for (direction, kind) in [
        (Direction::Render, SourceKind::SystemOutput),
        (Direction::Capture, SourceKind::Microphone),
    ] {
        let default = enumerator
            .get_default_device(&direction)
            .and_then(|device| device.get_id())
            .ok();
        let devices = enumerator
            .get_device_collection(&direction)
            .map_err(unavailable)?;
        for device in &devices {
            let device = device.map_err(unavailable)?;
            let id = device.get_id().map_err(unavailable)?;
            let is_default = default.as_ref() == Some(&id);
            let name = device.get_friendlyname().map_err(unavailable)?;
            endpoints.push(Endpoint {
                id,
                name,
                kind,
                is_default,
            });
        }
    }
    Ok(endpoints)
}

struct Frames<'a> {
    producer: &'a Producer,
    data: [i16; SAMPLES],
    used: usize,
    start_ms: u64,
    dropped_ms: Option<u64>,
}

impl Frames<'_> {
    fn push(&mut self, sample: i16) -> Result<(), AudioError> {
        self.data[self.used] = sample;
        self.used += 1;
        if self.used == SAMPLES {
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), AudioError> {
        self.data[self.used..].fill(0);
        let end_ms = self.start_ms + FRAME_MS;
        self.producer.push(
            self.data,
            Span {
                start_ms: self.start_ms,
                end_ms,
            },
            self.dropped_ms.take(),
        )?;
        self.start_ms = end_ms;
        self.used = 0;
        Ok(())
    }
}

pub(crate) fn capture(
    source: Source,
    clock: Instant,
    stop: &Stop,
    producer: &Producer,
    report: &impl Fn(AudioEvent),
    lost_ms: Option<u64>,
) -> Result<(), AudioError> {
    let _apartment = Apartment::enter()?;
    let direction = match source.kind {
        SourceKind::SystemOutput => Direction::Render,
        SourceKind::Microphone => Direction::Capture,
        _ => return Err(AudioError::new("Select a Windows output or microphone.")),
    };
    let enumerator = DeviceEnumerator::new().map_err(unavailable)?;
    let device = match &source.endpoint_id {
        Some(id) => enumerator.get_device(id),
        None => enumerator.get_default_device(&direction),
    }
    .map_err(unavailable)?;
    if device.get_direction() != direction
        || device.get_state().map_err(unavailable)? != DeviceState::Active
    {
        return Err(AudioError::device(
            "The selected endpoint is inactive or has a different audio direction.",
        ));
    }
    let id = device.get_id().map_err(unavailable)?;
    let mut client = device.get_iaudioclient().map_err(unavailable)?;
    let mix = client.get_mixformat().map_err(unavailable)?;
    let channels = usize::from(mix.get_nchannels());
    if !(1..=32).contains(&channels) {
        return Err(AudioError::new(
            "The selected endpoint has an unsupported channel layout.",
        ));
    }
    let format = WaveFormat::new(
        16,
        16,
        &SampleType::Int,
        SAMPLE_RATE as usize,
        channels,
        Some(mix.get_dwchannelmask()),
    );
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: 0,
    };
    client
        .initialize_client(&format, &Direction::Capture, &mode)
        .map_err(unavailable)?;
    let event = client.set_get_eventhandle().map_err(unavailable)?;
    // SAFETY: the shared-mode client is initialized for capture and owned by this COM thread.
    let capture: IAudioCaptureClient =
        unsafe { client.as_iaudioclient().GetService() }.map_err(unavailable)?;
    if stop.requested() {
        return Ok(());
    }
    client.start_stream().map_err(unavailable)?;
    let follow_default = source.endpoint_id.is_none();
    report(AudioEvent::Ready {
        source: Source {
            endpoint_id: Some(id.clone()),
            ..source
        },
    });
    let mut frames = Frames {
        producer,
        data: [0; SAMPLES],
        used: 0,
        start_ms: elapsed_ms(clock),
        // After a reconnect the first frame reports how long audio was lost.
        dropped_ms: lost_ms,
    };
    let mut next_check = frames.start_ms + 1_000;
    let mut last_packet = None;
    let mut last_driver_end = None;
    let result = (|| {
        while !stop.requested() {
            match event.wait_for_event(40) {
                Ok(()) | Err(WasapiError::EventTimeout) => {}
                Err(error) => return Err(unavailable(error)),
            }
            if stop.requested() {
                break;
            }
            let now = elapsed_ms(clock);
            if now >= next_check {
                next_check = now + 1_000;
                if device.get_state().map_err(unavailable)? != DeviceState::Active {
                    return Err(AudioError::device(
                        "The selected endpoint disconnected. Refresh devices and restart capture.",
                    ));
                }
                if follow_default
                    && enumerator
                        .get_default_device(&direction)
                        .and_then(|d| d.get_id())
                        .map_err(unavailable)?
                        != id
                {
                    return Err(AudioError::device(
                        "Windows changed the default audio device. Restart capture to use it.",
                    ));
                }
            }
            // SAFETY: all reads and packet releases are paired on the capture thread.
            while !stop.requested()
                && unsafe { capture.GetNextPacketSize() }.map_err(unavailable)? > 0
            {
                let mut data = std::ptr::null_mut();
                let (mut count, mut flags, mut timestamp) = (0, 0, 0);
                // SAFETY: out pointers live until the call returns. The WASAPI lease is released below.
                unsafe {
                    capture.GetBuffer(
                        &mut data,
                        &mut count,
                        &mut flags,
                        None,
                        Some(&mut timestamp),
                    )
                }
                .map_err(unavailable)?;
                if count == 0 {
                    break;
                }
                let packet = (|| {
                    let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
                    let discontinuity =
                        flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0;
                    let timestamp_valid = flags & AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR.0 as u32 == 0;
                    if discontinuity && last_packet.is_some_and(|at| now.saturating_sub(at) < 150) {
                        let gap = last_driver_end
                            .filter(|_| timestamp_valid)
                            .map(|end| timestamp.saturating_sub(end) / 10_000);
                        let Some(short) = gap.filter(|ms| *ms <= FRAME_MS) else {
                            return Err(AudioError { message: "Windows reported an audio discontinuity. Restart capture to resume cleanly.".into(), dropped_ms: gap.unwrap_or(0), device: true });
                        };
                        // A flagged packet that follows the last one without a gap lost nothing.
                        if short > 0 {
                            frames.dropped_ms = Some(frames.dropped_ms.unwrap_or(0) + short);
                        }
                        for _ in 0..short * u64::from(SAMPLE_RATE) / 1_000 {
                            frames.push(0)?;
                        }
                    }
                    last_packet = Some(now);
                    last_driver_end = timestamp_valid.then_some(
                        timestamp + u64::from(count) * 10_000_000 / u64::from(SAMPLE_RATE),
                    );
                    if silent {
                        for _ in 0..count {
                            frames.push(0)?;
                        }
                    } else if data.is_null() {
                        return Err(AudioError::device(
                            "Windows returned an invalid audio buffer.",
                        ));
                    } else {
                        // SAFETY: a non-silent packet contains count frames in the negotiated PCM format.
                        let bytes = unsafe {
                            std::slice::from_raw_parts(data, count as usize * channels * 2)
                        };
                        for frame in bytes.chunks_exact(channels * 2) {
                            let sum: i32 = frame
                                .as_chunks::<2>()
                                .0
                                .iter()
                                .map(|s| i32::from(i16::from_le_bytes(*s)))
                                .sum();
                            frames.push((sum / channels as i32) as i16)?;
                        }
                    }
                    Ok(())
                })();
                // SAFETY: this releases exactly the nonempty packet acquired above.
                let release = unsafe { capture.ReleaseBuffer(count) }.map_err(unavailable);
                packet.and(release)?;
            }
            // A loopback endpoint may stop delivering packets when the output becomes idle.
            // Insert timed silence so the pending utterance can still reach its endpoint.
            if now.saturating_sub(last_packet.unwrap_or(0)) >= 100 {
                if now.saturating_sub(frames.start_ms) > 2_000 {
                    return Err(AudioError::device(
                        "Audio clock jumped. Restart capture after waking the device.",
                    ));
                }
                while frames.start_ms + FRAME_MS <= now.saturating_sub(70) {
                    frames.flush()?;
                }
            }
        }
        if frames.used > 0 {
            frames.flush()?;
        }
        Ok(())
    })();
    result.and(client.stop_stream().map_err(unavailable))
}
