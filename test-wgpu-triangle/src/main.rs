use std::path::Path;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
const TOLERANCE: u8 = 1;

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let generate = args.get(1).map(|s| s.as_str()) == Some("generate");

    println!("Rendering {WIDTH}x{HEIGHT} triangle...");
    let pixels = pollster::block_on(render_triangle());

    let output_path = Path::new("output.png");
    save_png(output_path, &pixels);
    println!("Saved output.png");

    if generate {
        let expected_path = Path::new("snapshots/expected.png");
        std::fs::create_dir_all("snapshots").unwrap();
        save_png(expected_path, &pixels);
        println!("Generated snapshots/expected.png");
    } else {
        let expected_path = Path::new("snapshots/expected.png");
        if !expected_path.exists() {
            eprintln!("Error: snapshots/expected.png not found.");
            eprintln!("Run with 'generate' argument to create the snapshot first.");
            std::process::exit(1);
        }
        let expected = load_png(expected_path);
        compare_images(&pixels, &expected);
        println!("PASS: Output matches expected snapshot.");
    }
}

async fn render_triangle() -> Vec<u8> {
    // Create wgpu instance with Vulkan backend
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    // Request adapter (prefer software/fallback for lavapipe)
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            compatible_surface: None,
            force_fallback_adapter: true,
        })
        .await
        .expect("Failed to find a Vulkan adapter. Is VK_DRIVER_FILES set and lavapipe available?");

    let info = adapter.get_info();
    println!(
        "Using adapter: {} (backend: {:?}, type: {:?})",
        info.name, info.backend, info.device_type
    );

    // Request device
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("Failed to create device");

    // Create render target texture
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("render target"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Create shader module
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    // Create render pipeline
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pipeline layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("render pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    // Calculate buffer layout with alignment
    let bytes_per_pixel = 4u32;
    let unpadded_bytes_per_row = WIDTH * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;
    let buffer_size = (padded_bytes_per_row * HEIGHT) as u64;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Record and submit render commands
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        render_pass.set_pipeline(&pipeline);
        render_pass.draw(0..3, 0..1);
    }

    // Copy texture to buffer
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(Some(encoder.finish()));

    // Read back pixels
    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();

    let data = buffer_slice.get_mapped_range();

    // Remove row padding
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * bytes_per_pixel) as usize);
    for row in 0..HEIGHT {
        let start = (row * padded_bytes_per_row) as usize;
        let end = start + (WIDTH * bytes_per_pixel) as usize;
        pixels.extend_from_slice(&data[start..end]);
    }

    drop(data);
    output_buffer.unmap();

    pixels
}

fn save_png(path: &Path, pixels: &[u8]) {
    let img = image::RgbaImage::from_raw(WIDTH, HEIGHT, pixels.to_vec())
        .expect("Failed to create image from pixels");
    img.save(path)
        .unwrap_or_else(|e| panic!("Failed to save {}: {e}", path.display()));
}

fn load_png(path: &Path) -> Vec<u8> {
    let img = image::open(path)
        .unwrap_or_else(|e| panic!("Failed to load {}: {e}", path.display()))
        .to_rgba8();
    assert_eq!(
        img.width(),
        WIDTH,
        "Expected width {WIDTH}, got {}",
        img.width()
    );
    assert_eq!(
        img.height(),
        HEIGHT,
        "Expected height {HEIGHT}, got {}",
        img.height()
    );
    img.into_raw()
}

fn compare_images(actual: &[u8], expected: &[u8]) {
    assert_eq!(actual.len(), expected.len(), "Image byte sizes don't match");

    let mut max_diff: u8 = 0;
    let mut diff_count: usize = 0;

    for (a, e) in actual.iter().zip(expected.iter()) {
        let diff = (*a as i16 - *e as i16).unsigned_abs() as u8;
        if diff > TOLERANCE {
            diff_count += 1;
            if diff > max_diff {
                max_diff = diff;
            }
        }
    }

    if diff_count > 0 {
        let total_pixels = (WIDTH * HEIGHT) as usize;
        let diff_channels = diff_count;
        eprintln!("FAIL: Images differ!");
        eprintln!("  Differing channels: {diff_channels} / {}", total_pixels * 4);
        eprintln!("  Max channel difference: {max_diff} (tolerance: {TOLERANCE})");
        std::process::exit(1);
    }
}
