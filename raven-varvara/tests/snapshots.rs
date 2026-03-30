use image::{DynamicImage, ImageBuffer, ImageReader, Rgba};
use raven_varvara::Varvara;
use std::io::Read;
use std::path::Path;
use uxn::{Backend, Uxn, UxnMem, backend};

struct Snapshot {
    pixels: Vec<u8>,
    size: (u16, u16),
}

fn get_snapshot<B: uxn::Backend>(
    rom: &[u8],
) -> Result<Snapshot, std::io::Error> {
    let mut mem = UxnMem::boxed();
    let mut vm = Uxn::<B>::new(&mut mem);
    let mut dev = Varvara::new();
    let data = vm.reset(rom);
    dev.reset(data);
    vm.run(&mut dev, 0x100); // init vector
    let out = dev.output(&vm);
    out.check()?;
    let size = out.size;

    // Do some input!
    dev.mouse(
        &mut vm,
        raven_varvara::MouseState {
            pos: (size.0 as f32 / 2.0, size.1 as f32 / 2.0),
            buttons: 1,
            scroll: (0.0, 0.0),
        },
    );
    dev.pressed(&mut vm, raven_varvara::Key::Right, false);
    dev.pressed(&mut vm, raven_varvara::Key::Char(b'a'), false);
    for _ in 0..60 {
        dev.redraw(&mut vm);
    }
    let out = dev.output(&vm);
    out.check()?;

    // BGRA -> RGBA
    let mut pixels = out.frame.to_owned();
    for chunk in pixels.chunks_mut(4) {
        chunk.swap(0, 2);
    }
    Ok(Snapshot {
        pixels,
        size: out.size,
    })
}

fn run_and_check<B: Backend>(name: &str) {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let rom_path = Path::new(&manifest_dir)
        .parent()
        .expect("missing parent directory")
        .join(format!("roms/{name}.rom"));
    let mut rom = vec![];
    std::fs::File::open(&rom_path)
        .expect("could not open ROM file")
        .read_to_end(&mut rom)
        .expect("failed to read ROM");
    let snapshot = get_snapshot::<B>(&rom).expect("ROM execution failed");

    let our_image = ImageBuffer::<Rgba<u8>, _>::from_raw(
        snapshot.size.0 as u32,
        snapshot.size.1 as u32,
        snapshot.pixels,
    )
    .expect("Failed to create image buffer");

    let output_path = Path::new(&manifest_dir)
        .join(format!("tests/{}.png", name.replace(".", "_")));

    if output_path.exists() {
        let DynamicImage::ImageRgba8(image) = ImageReader::open(&output_path)
            .expect("failed to open on-disk image")
            .decode()
            .expect("failed to decode on-disk image")
        else {
            panic!("on-disk image is of an invalid type");
        };

        // Compare pixel data, building a comparison image
        const PADDING: u32 = 20;
        let width = snapshot.size.0 as u32;
        let height = snapshot.size.1 as u32;
        let stride = width * 3 + PADDING * 4;
        let mut out =
            ImageBuffer::<Rgba<u8>, _>::new(stride, height + PADDING * 2);
        let mut failed = false;
        for y in 0..height {
            for x in 0..width {
                out[(x + PADDING, y + PADDING)] = image[(x, y)];
                out[(x + 2 * width + 3 * PADDING, y + PADDING)] =
                    our_image[(x, y)];
                out[(x + 2 * PADDING + width, y + PADDING)] =
                    if our_image[(x, y)] != image[(x, y)] {
                        failed = true;
                        Rgba([0xFF, 0, 0, 0xFF])
                    } else {
                        Rgba([0xFF; 4])
                    };
            }
        }
        if failed {
            let fail_path = Path::new(&manifest_dir)
                .join(format!("tests/{}.failed.png", name.replace(".", "_")));
            out.save(&fail_path)
                .expect("Failed to save the failure PNG file");
            panic!("image mismatch in {name}, saved to {fail_path:?}");
        }
    } else {
        our_image
            .save(&output_path)
            .expect("Failed to save the PNG file");
    }
}

mod snapshots {
    use super::*;

    macro_rules! snapshot_test {
        ($name:ident, $backend:path) => {
            #[test]
            fn $name() {
                run_and_check::<$backend>(stringify!($name));
            }
        };
    }
    macro_rules! snapshot_tests {
        ($backend:path) => {
            snapshot_test!(audio, $backend);
            snapshot_test!(controller, $backend);
            snapshot_test!(piano, $backend);
            snapshot_test!(mandelbrot, $backend);
            snapshot_test!(screen_auto, $backend);
            snapshot_test!(screen_blending, $backend);
            snapshot_test!(screen_bounds, $backend);
            snapshot_test!(screen_pixel, $backend);
            snapshot_test!(screen, $backend);
        };
    }
    snapshot_tests!(backend::Interpreter);

    #[cfg(feature = "native")]
    mod native {
        use super::*;
        snapshot_tests!(backend::Native);
    }

    #[cfg(feature = "tailcall")]
    mod tailcall {
        use super::*;
        snapshot_tests!(backend::Tailcall);
    }
}
