extern crate image;
extern crate encoding;
extern crate rand;

use std::env;
use std::io;
use std::convert::TryInto;
use image::codecs::gif::GifEncoder;
use image::{Delay, Frame};
use rusttype::{Scale, Font};
use encoding::{Encoding, EncoderTrap, DecoderTrap};
use encoding::all::MAC_CYRILLIC;

const SCALE_X: usize = 32;
const SCALE_Y: usize = 32;
const SCALE_X_FLOAT: f32 = 32.0;
const SCALE_Y_FLOAT: f32 = 32.0;

fn update_random_chars(chars: &mut Vec<Vec<char>>, height: usize) {
    for i in 0..chars.len() {
        let rand_byte = rand::random::<u8>();
        let rand_char = MAC_CYRILLIC.decode(&[rand_byte], DecoderTrap::Replace).expect("Decoding error").chars().nth(0).expect("Empty random char");
        chars[i].insert(0, rand_char);
        chars[i].truncate(height);
    }
}

fn compare_chars(random_chars: &mut Vec<Vec<char>>, s_chars: &mut Vec<Vec<char>>, revealed_chars: &mut Vec<Vec<char>>) -> bool {
    let mut has_nonrevealed_chars = false;
    for row_i in 0..random_chars.len() {
        for ch_i in 0..random_chars[row_i].len() {
            let s_char = s_chars[row_i][ch_i];
            let random_char = random_chars[row_i][ch_i];
            let revealed_char = revealed_chars[row_i][ch_i];
            if s_char != '\0' {
                if random_char == s_char {
                    revealed_chars[row_i][ch_i] = s_char;
                } else if s_char != revealed_char {
                    has_nonrevealed_chars = true;
                }
            }
        }
    }

    return has_nonrevealed_chars;
}

fn draw_frame(canvas: &mut image::RgbaImage, bgchars: &Vec<Vec<char>>, fgchars: &Vec<Vec<char>>, font: &Font) {
    let color_fg = image::Rgba([0, 255, 0, 255]);
    let color_bg = image::Rgba([0, 160, 0, 255]);
    let scale = Scale{ x: SCALE_X_FLOAT, y: SCALE_Y_FLOAT };
    for bg_i in 0..bgchars.len() {
        for ch_i in 0..bgchars[bg_i].len() {
            let bgchar = bgchars[bg_i][ch_i];
            let fgchar = fgchars[bg_i][ch_i];
            let is_fg = if fgchar != '\0' { true } else { false };
            if is_fg || bgchar != '\0' {
                imageproc::drawing::draw_text_mut(
                    canvas,
                    if is_fg { color_fg } else { color_bg },
                    (bg_i * SCALE_X).try_into().unwrap(),
                    (ch_i * SCALE_Y).try_into().unwrap(),
                    scale,
                    font,
                    &(if is_fg { fgchar } else { bgchar }).to_string()
                );
            }
        }
    }
}

fn generate_frames(s: String) -> Vec<Frame> {
    let encoded_s = MAC_CYRILLIC.encode(&s, EncoderTrap::Replace).expect("Initial encoding error");
    let decoded_s = MAC_CYRILLIC.decode(&encoded_s, DecoderTrap::Replace).expect("Initial decoding error");

    let s_rows = decoded_s.split(" ").collect::<Vec<&str>>();
    let s_width = s_rows.iter().map(|s| { s.chars().count() }).fold(usize::MIN, usize::max);
    let s_height = s_rows.len();
    let s_height_full = s_height + 5;
    let frame_width = (s_width * SCALE_X).try_into().unwrap();
    let frame_height = (s_height_full * SCALE_Y).try_into().unwrap();

    let font_data: &[u8] = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf");
    let font = Font::try_from_bytes(font_data).expect("Failed to load font");
    let color_black = image::Rgba([0, 0, 0, 255]);

    let mut random_chars: Vec<Vec<char>> = vec![vec!['\0'; s_height_full]; s_width];
    let mut s_chars: Vec<Vec<char>> = random_chars.clone();
    let mut revealed_chars: Vec<Vec<char>> = s_chars.clone();

    for fg_i in 0..s_rows.len() {
        for (ch_i, ch) in s_rows[fg_i].chars().enumerate() {
            s_chars[ch_i][fg_i + 2] = ch;
        }
    }

    let mut frames: Vec<Frame> = vec![];
    loop {
        update_random_chars(&mut random_chars, s_height_full);
        let has_nonrevealed_chars = compare_chars(&mut random_chars, &mut s_chars, &mut revealed_chars);

        let mut buf = image::RgbaImage::from_pixel(frame_width, frame_height, color_black);
        draw_frame(&mut buf, &random_chars, &revealed_chars, &font);
        let frame = Frame::from_parts(buf, 0, 0, Delay::from_numer_denom_ms(10, 1));
        frames.push(frame);

        if !has_nonrevealed_chars {
            break;
        }
    }

    return frames;
}

fn main() {
    let args = env::args();
    let s = args.skip(1).next()
        .expect("String not provided");

    let frames = generate_frames(s);
    let stdout = io::stdout();
    let mut encoder = GifEncoder::new(stdout);
    encoder.encode_frames(frames);
}
