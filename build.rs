use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets/original-sprites.png");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut decoder = png::Decoder::new(std::io::Cursor::new(include_bytes!(
        "assets/original-sprites.png"
    )));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().expect("original sprite PNG");
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).unwrap();
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => panic!("RGB sprites required"),
    };
    let palette: [u32; 16] = [
        0, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555,
        0x5555ff, 0x55ff55, 0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff,
    ];
    let mut sprite = [0u32; 256];
    for y in 0..16 {
        for x in 0..16 {
            // Third locomotive cell: original right-facing blue steam engine.
            let offset = ((160 + y) * info.width as usize + 32 + x) * channels;
            let p = &pixels[offset..offset + 3];
            sprite[y * 16 + x] = *palette
                .iter()
                .min_by_key(|c| {
                    let r = ((**c >> 16) & 255) as i32 - p[0] as i32;
                    let g = ((**c >> 8) & 255) as i32 - p[1] as i32;
                    let b = (**c & 255) as i32 - p[2] as i32;
                    r * r + g * g + b * b
                })
                .unwrap();
        }
    }
    // ICO contains true-size, nearest-neighbor variants with transparent black.
    let sizes = [16usize, 32, 48, 64, 128, 256];
    let mut icon = Vec::new();
    icon.extend([0, 0, 1, 0]);
    icon.extend((sizes.len() as u16).to_le_bytes());
    let mut payloads = Vec::new();
    let mut offset = 6 + sizes.len() * 16;
    for size in sizes {
        let mask_stride = size.div_ceil(32) * 4;
        let mut image = Vec::new();
        image.extend(40u32.to_le_bytes());
        image.extend((size as i32).to_le_bytes());
        image.extend(((size * 2) as i32).to_le_bytes());
        image.extend(1u16.to_le_bytes());
        image.extend(32u16.to_le_bytes());
        image.extend(0u32.to_le_bytes());
        image.extend(((size * size * 4 + mask_stride * size) as u32).to_le_bytes());
        image.extend([0; 16]);
        for y in (0..size).rev() {
            for x in 0..size {
                let color = sprite[(y * 16 / size) * 16 + x * 16 / size];
                image.extend([
                    color as u8,
                    (color >> 8) as u8,
                    (color >> 16) as u8,
                    if color == 0 { 0 } else { 255 },
                ]);
            }
        }
        for y in (0..size).rev() {
            let mut mask = vec![0; mask_stride];
            for x in 0..size {
                if sprite[(y * 16 / size) * 16 + x * 16 / size] == 0 {
                    mask[x / 8] |= 0x80 >> (x % 8);
                }
            }
            image.extend(mask);
        }
        icon.extend([size as u8, size as u8, 0, 0]);
        icon.extend(1u16.to_le_bytes());
        icon.extend(32u16.to_le_bytes());
        icon.extend((image.len() as u32).to_le_bytes());
        icon.extend((offset as u32).to_le_bytes());
        offset += image.len();
        payloads.push(image);
    }
    for image in payloads {
        icon.extend(image);
    }
    let icon_path = out.join("vlak.ico");
    fs::write(&icon_path, icon).unwrap();
    let rc_path = out.join("vlak.rc");
    fs::write(
        &rc_path,
        format!(
            "1 ICON \"{}\"\n",
            icon_path.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    embed_resource::compile(&rc_path, embed_resource::NONE)
        .manifest_required()
        .expect("Windows icon resource compilation");
}
