#![feature(binary_heap_into_iter_sorted)]

//use image::{Rgb, RgbImage, GrayImage, Luma};
//use imageproc::drawing::{draw_text_mut};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use rusttype::{Font, Point, Scale};
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::sync::Mutex;
use walkdir::WalkDir;

/// Output file has the following format:
///  - One byte storing the length in 10ths of an `m` of all omitted characters.
///  - For each character (sorted by character)
///     - Character in UTF-8
///     - Length in 10ths of an `m` as a byte
fn main() {
    let fonts: Vec<Font> = WalkDir::new("./src/ttf")
        .into_iter()
        .map(|r| r.unwrap())
        .filter(|d| d.path().extension() == Some(OsStr::new("ttf")))
        .map(|d| {
            let bytes = std::fs::read(d.path()).unwrap();
            Font::try_from_vec(bytes).unwrap()
        })
        .collect();

    struct Output {
        histogram: [usize; 256],
        tab: Vec<(char, u8)>,
    }

    impl Output {
        pub fn push(&mut self, c: char, max_width: u8) {
            self.histogram[max_width as usize] += 1;
            self.tab.push((c, max_width));
        }
    }

    let output = Mutex::new(Output {
        histogram: [0; 256],
        tab: Vec::new(),
    });

    (0..=char::MAX as u32).into_par_iter().for_each(|u| {
        if let Some(c) = char::from_u32(u) {
            let max_width = (max_width(c, &fonts) as f32 / 100f32).round() as u16;
            if max_width > u8::MAX as u16 {
                panic!("{}", c);
            }
            let max_width = max_width as u8;

            output.lock().unwrap().push(c, max_width);

            //println!("{} -> {}", c, max_width);
        }
    });

    let mut output = output.into_inner().unwrap();

    output.tab.sort_by_key(|&(c, _)| c);

    let mut mode = 0;
    let mut mode_n = 0;
    for (i, &n) in output.histogram.iter().enumerate() {
        let i = i as u8;
        println!("{}, {}", i, n);
        if n > mode_n {
            mode = i;
            mode_n = n;
        }
    }

    println!("Mode: {}", mode);

    let output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open("./src/character_widths.bin")
        .unwrap();
    let mut buffered = BufWriter::new(output_file);

    buffered.write_all(&[mode]).unwrap();

    for (c, max_width) in output.tab {
        if max_width == mode {
            continue;
        }
        let mut tmp = [0u8; 4];
        let s = c.encode_utf8(&mut tmp);
        buffered.write_all(s.as_bytes()).unwrap();
        buffered.write_all(&[max_width as u8]).unwrap();
    }

    buffered.flush().unwrap();

    /*
    const RESOLUTION: u32 = 32;

    let path = Path::new(&arg);

    let mut image = GrayImage::new(RESOLUTION, RESOLUTION);

    let height = RESOLUTION as f32;
    let scale = Scale {
        x: height,
        y: height,
    };

    let text = "\u{12345}";
    draw_text_mut(&mut image, Luma([255u8]), 0, 0, scale, &font, text);

    let _ = image.save(path).unwrap();
     */
}

/// Computes max width in milli-m's.
fn max_width(c: char, fonts: &[Font]) -> usize {
    let mut max_width = 0;
    for font in fonts {
        let width = width(c, font);
        max_width = max_width.max(width);
    }
    max_width
}

/// Computes with in milli-m's.
fn width(c: char, font: &Font) -> usize {
    let mut tmp = [0u8; 4];
    let s = c.encode_utf8(&mut tmp);

    let mut min = i32::MAX;
    let mut max = i32::MIN;

    font.layout(s, Scale::uniform(1344.0), Point::default())
        .filter_map(|i| i.pixel_bounding_box())
        .for_each(|b| {
            min = min.min(b.min.x);
            max = max.max(b.max.x);
        });

    max.checked_sub(min).unwrap_or(0) as usize
}
