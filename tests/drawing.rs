//! Draws the plug-in's real scene with the player's real renderer.
//!
//! `LOTUS_PREVIEW=/some/dir cargo test -- --ignored preview` writes the
//! flower at rest and in full swell there, as PPM pictures.

use popamp_plugin_lotus::{pulse::RINGS, scene_json};
use popamp_visual::{wgpu, Frame, Gpu, Renderer, Scene, SPECTRUM_LEN, WAVE_LEN};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 256;
const PREVIEW_SIZE: u32 = 768;
const DT: f32 = 1.0 / 60.0;
const WAVE: [f32; WAVE_LEN] = [0.0; WAVE_LEN];
const SPECTRUM: [f32; SPECTRUM_LEN] = [0.0; SPECTRUM_LEN];
const AT_REST: Music = Music { level: 0.0, kick_age: 1000.0 };
const IN_FULL_SWELL: Music = Music { level: 1.0, kick_age: 0.08 };
/// The pixel brightness from which a pixel counts as flower, not background.
const LIT: u8 = 20;

/// What pulse.rs would hand the shader: every ring at one level, and a kick of full strength.
struct Music {
    level: f32,
    kick_age: f32,
}

impl Music {
    fn data(&self) -> Vec<u8> {
        let floats = [self.level; RINGS].into_iter().chain([self.kick_age, 1.0, 0.0]);
        floats.flat_map(f32::to_le_bytes).collect()
    }
}

struct Stage {
    gpu: Gpu,
    renderer: Renderer,
    output: wgpu::Texture,
    size: u32,
}

impl Stage {
    /// No GPU at all (a bare build machine) skips the test with a note.
    fn new(size: u32) -> Option<Stage> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let gpu = match Gpu::new(&instance, None) {
            Ok(gpu) => gpu,
            Err(e) => {
                eprintln!("skipped: {e}");
                return None;
            }
        };
        let scene = Scene::parse(scene_json().as_bytes()).unwrap();
        let renderer = Renderer::new(&gpu, scene, FORMAT, (size, size)).unwrap();
        let output = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: extent(size),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Some(Stage { gpu, renderer, output, size })
    }

    /// The picture as RGBA rows.
    fn draw(&mut self, music: &Music) -> Vec<u8> {
        let view = self.output.create_view(&Default::default());
        let data = music.data();
        let frame = Frame { time: 1.0, dt: DT, wave: &WAVE, spectrum: &SPECTRUM, data: &data };
        self.renderer.draw(&self.gpu, &view, &frame).unwrap();

        let bytes_per_row = self.size * 4;
        let buffer = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(bytes_per_row * self.size),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        let layout = wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bytes_per_row), rows_per_image: None };
        encoder.copy_texture_to_buffer(
            self.output.as_image_copy(),
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout },
            extent(self.size),
        );
        self.gpu.queue.submit([encoder.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |r| r.unwrap());
        self.gpu.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = buffer.slice(..).get_mapped_range().unwrap().to_vec();
        pixels
    }
}

fn extent(size: u32) -> wgpu::Extent3d {
    wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 }
}

/// The brightest colour channel of each pixel.
fn brightness(rgba: &[u8]) -> impl Iterator<Item = u8> + '_ {
    rgba.as_chunks::<4>().0.iter().map(|p| p[0].max(p[1]).max(p[2]))
}

fn lit_pixels(rgba: &[u8]) -> usize {
    brightness(rgba).filter(|&b| b > LIT).count()
}

fn at(rgba: &[u8], (x, y): (u32, u32)) -> &[u8] {
    let i = ((y * SIZE + x) * 4) as usize;
    &rgba[i..i + 3]
}

#[test]
fn the_player_accepts_the_scene() {
    Scene::parse(scene_json().as_bytes()).unwrap();
}

#[test]
fn silence_draws_a_flower_with_a_golden_heart_on_a_dark_ground() {
    let Some(mut stage) = Stage::new(SIZE) else { return };
    let picture = stage.draw(&AT_REST);
    let [r, g, b] = at(&picture, (SIZE / 2, SIZE / 2)).try_into().unwrap();
    assert!(r > g && g > b && r > 80, "the heart is not golden: {r} {g} {b}");
    assert!(brightness(at(&picture, (0, 0))).all(|c| c < LIT), "a corner is lit");
    assert!(lit_pixels(&picture) > (SIZE * SIZE / 2) as usize, "the flower is too small");
}

#[test]
fn music_makes_the_flower_bigger() {
    let Some(mut stage) = Stage::new(SIZE) else { return };
    let rest = lit_pixels(&stage.draw(&AT_REST));
    let swell = lit_pixels(&stage.draw(&IN_FULL_SWELL));
    assert!(swell as f32 > rest as f32 * 1.1, "{rest} pixels at rest, {swell} in full swell");
}

#[test]
#[ignore = "writes pictures to look at"]
fn preview() {
    let dir = std::env::var("LOTUS_PREVIEW").expect("LOTUS_PREVIEW names the folder for the pictures");
    let Some(mut stage) = Stage::new(PREVIEW_SIZE) else { return };
    for (name, music) in [("rest", AT_REST), ("swell", IN_FULL_SWELL)] {
        let rgb: Vec<u8> = stage.draw(&music).as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
        let mut ppm = format!("P6\n{PREVIEW_SIZE} {PREVIEW_SIZE}\n255\n").into_bytes();
        ppm.extend(rgb);
        std::fs::write(format!("{dir}/{name}.ppm"), ppm).unwrap();
    }
}
