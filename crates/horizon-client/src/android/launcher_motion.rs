//! Muted launcher animation prepared from the supplied video at 480×270, 12 fps.
//! Each JPEG is prefixed with its little-endian u32 length for direct frame access.
use horizon_client::{font::Patch, hud::HudAtlas};
use std::time::Instant;

const VIDEO: &[u8] = include_bytes!("mybezo-motion.bin");
pub(super) const FRAME_MS: u64 = 83;

pub(super) struct Motion {
    frames: Vec<&'static [u8]>,
    start: Option<Instant>,
    uploaded: Option<usize>,
}

impl Motion {
    pub(super) fn new() -> Self {
        let mut frames = Vec::new();
        let mut data = VIDEO;
        while let Some((length, rest)) = data.split_first_chunk::<4>() {
            let len = u32::from_le_bytes(*length) as usize;
            let Some((frame, tail)) = rest.split_at_checked(len) else {
                break;
            };
            frames.push(frame);
            data = tail;
        }
        Self { frames, start: None, uploaded: None }
    }

    pub(super) fn pause(&mut self) {
        self.start = None;
    }

    pub(super) fn invalidate(&mut self) {
        self.uploaded = None;
    }

    pub(super) fn patch(&mut self, atlas: &HudAtlas, sprite: &str) -> Option<Patch> {
        if self.frames.is_empty() {
            return None;
        }
        let elapsed = self.start.get_or_insert_with(Instant::now).elapsed();
        let index = (elapsed.as_millis() * 12 / 1000 % self.frames.len() as u128) as usize;
        if self.uploaded == Some(index) {
            return None;
        }
        let rect = atlas.sprites.get(sprite)?;
        let image = image::load_from_memory_with_format(self.frames[index], image::ImageFormat::Jpeg)
            .ok()?
            .into_rgba8();
        if image.dimensions() != (rect.w, rect.h) {
            return None;
        }
        self.uploaded = Some(index);
        Some(Patch {
            x: (rect.u0 * atlas.width as f32).round() as u32,
            y: (rect.v0 * atlas.height as f32).round() as u32,
            width: rect.w,
            height: rect.h,
            rgba: image.into_raw(),
        })
    }
}
