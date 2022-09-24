extern crate image;
extern crate encoding;
extern crate rand;
extern crate arguments;

use std::env;
use std::io;
use std::convert::TryInto;
use image::codecs::gif::GifEncoder;
use image::{Delay, Frame, Rgba};
use rusttype::{Scale, Font};
use encoding::{Encoding, EncoderTrap, DecoderTrap};
use encoding::all::MAC_CYRILLIC;
use rand::seq::SliceRandom;
use imageproc::rect::Rect;

#[derive(Clone, Copy)]
struct ColorChar {
    ch: char,
    color: Rgba<u8>,
}

const SCALE_X: usize = 32;
const SCALE_Y: usize = 32;
const OFFSET_X: usize = 8;
const SCALE_X_U32: u32 = 32;
const SCALE_Y_U32: u32 = 32;
const SCALE_X_FLOAT: f32 = 32.0;
const SCALE_Y_FLOAT: f32 = 32.0;

const COLORS_DIM: [Rgba<u8>;5] = [
    Rgba([0xff, 0xd1, 0x75, 0xff]),
    Rgba([0x75, 0xff, 0xc6, 0xff]),
    Rgba([0x75, 0xdd, 0xff, 0xff]),
    Rgba([0x81, 0x75, 0xff, 0xff]),
    Rgba([0xdd, 0x75, 0xff, 0xff]),
];

fn darken(color: Rgba<u8>) -> Rgba<u8> {
    Rgba([
        color[0] / 10 * 6,
        color[1] / 10 * 6,
        color[2] / 10 * 6,
        color[3],
    ])
}

fn random_color_excluded(excluded: Rgba<u8>) -> Rgba<u8> {
    let mut rng = rand::thread_rng();
    loop {
        let color = *(COLORS_DIM.choose(&mut rng).unwrap());
        if color != excluded {
            return color;
        }
    }
}

fn update_random_chars(chars: &mut Vec<Vec<ColorChar>>, height: usize, column_colors: &Vec<Rgba<u8>>) {
    for i in 0..chars.len() {
        let rand_byte = rand::random::<u8>();
        let rand_char = MAC_CYRILLIC.decode(&[rand_byte], DecoderTrap::Replace).expect("Decoding error").chars().nth(0).expect("Empty random char");
        let rand_color = random_color_excluded(column_colors[i]);
        chars[i].insert(0, ColorChar {
            ch: rand_char,
            color: rand_color,
        });
        chars[i].truncate(height);
    }
}

fn compare_chars(random_chars: &mut Vec<Vec<ColorChar>>, s_chars: &mut Vec<Vec<ColorChar>>, revealed_chars: &mut Vec<Vec<ColorChar>>) -> bool {
    let mut has_nonrevealed_chars = false;
    for row_i in 0..random_chars.len() {
        for ch_i in 0..random_chars[row_i].len() {
            let s_char = s_chars[row_i][ch_i];
            let random_char = random_chars[row_i][ch_i];
            let revealed_char = revealed_chars[row_i][ch_i];
            if s_char.ch != '\0' {
                if random_char.ch == s_char.ch {
                    revealed_chars[row_i][ch_i] = ColorChar {
                        ch: s_char.ch,
                        color: darken(random_char.color),
                    };
                } else if s_char.ch != revealed_char.ch {
                    has_nonrevealed_chars = true;
                }
            }
        }
    }

    return has_nonrevealed_chars;
}

fn draw_frame(canvas: &mut image::RgbaImage, bgchars: &Vec<Vec<ColorChar>>, fgchars: &Vec<Vec<ColorChar>>, font: &Font, color: bool, column_colors: &Vec<Rgba<u8>>) {
    if color {
        for (i, color) in column_colors.iter().enumerate() {
            imageproc::drawing::draw_filled_rect_mut(
                canvas,
                Rect::at((i * SCALE_X).try_into().unwrap(), 0)
                    .of_size(SCALE_X_U32, SCALE_Y_U32 * canvas.height()),
                *color,
            );
        }
    }

    let color_fg = Rgba([0, 255, 0, 255]);
    let color_bg = Rgba([0, 160, 0, 255]);
    let color_shadow = Rgba([0, 0, 0, 0]);
    let scale = Scale{ x: SCALE_X_FLOAT, y: SCALE_Y_FLOAT };
    for bg_i in 0..bgchars.len() {
        for ch_i in 0..bgchars[bg_i].len() {
            let bgchar = &bgchars[bg_i][ch_i];
            let fgchar = &fgchars[bg_i][ch_i];
            let is_fg = if fgchar.ch != '\0' { true } else { false };
            if is_fg || bgchar.ch != '\0' {
                let ch = (if is_fg { fgchar.ch } else { bgchar.ch }).to_string();
                if color {
                    imageproc::drawing::draw_text_mut(
                        canvas,
                        color_shadow,
                        (bg_i * SCALE_X + OFFSET_X + 1).try_into().unwrap(),
                        (ch_i * SCALE_Y + 1).try_into().unwrap(),
                        scale,
                        font,
                        &ch,
                    );
                }
                imageproc::drawing::draw_text_mut(
                    canvas,
                    if color {
                        if is_fg {
                            fgchar.color
                        } else {
                            bgchar.color
                        }
                    } else {
                        if is_fg {
                            color_fg
                        } else {
                            color_bg
                        }
                    },
                    (bg_i * SCALE_X + OFFSET_X).try_into().unwrap(),
                    (ch_i * SCALE_Y).try_into().unwrap(),
                    scale,
                    font,
                    &ch,
                );
            }
        }
    }
}

fn generate_frames(s: &str, color: bool) -> Vec<Frame> {
    let encoded_s = MAC_CYRILLIC.encode(s, EncoderTrap::Replace).expect("Initial encoding error");
    let decoded_s = MAC_CYRILLIC.decode(&encoded_s, DecoderTrap::Replace).expect("Initial decoding error");

    let s_rows = decoded_s.split(" ").collect::<Vec<&str>>();
    let s_width = s_rows.iter().map(|s| { s.chars().count() }).fold(usize::MIN, usize::max);
    let s_height = s_rows.len();
    let s_height_full = s_height + 5;
    let frame_width = (s_width * SCALE_X).try_into().unwrap();
    let frame_height = (s_height_full * SCALE_Y).try_into().unwrap();

    let mut column_colors: Vec<Rgba<u8>> = vec![];
    for i in 0..s_width {
        let excluded_color = match i {
            0 => Rgba([0, 0, 0, 0]),
            _ => column_colors[i-1],
        };
        column_colors.push(random_color_excluded(excluded_color));
    }

    let font_data: &[u8] = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf");
    let font = Font::try_from_bytes(font_data).expect("Failed to load font");
    let color_black = Rgba([0, 0, 0, 255]);

    let mut random_chars: Vec<Vec<ColorChar>> = vec![vec![ColorChar{
        ch: '\0',
        color: Rgba([0, 160, 0, 255]),
    }; s_height_full]; s_width];
    let mut s_chars: Vec<Vec<ColorChar>> = random_chars.clone();
    let mut revealed_chars: Vec<Vec<ColorChar>> = s_chars.clone();

    for fg_i in 0..s_rows.len() {
        for (ch_i, ch) in s_rows[fg_i].chars().enumerate() {
            s_chars[ch_i][fg_i + 2].ch = ch;
        }
    }

    let mut frames: Vec<Frame> = vec![];
    loop {
        update_random_chars(&mut random_chars, s_height_full, &column_colors);
        let has_nonrevealed_chars = compare_chars(&mut random_chars, &mut s_chars, &mut revealed_chars);

        let mut buf = image::RgbaImage::from_pixel(frame_width, frame_height, color_black);
        draw_frame(&mut buf, &random_chars, &revealed_chars, &font, color, &column_colors);
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
    let args = arguments::parse(args).unwrap();
    let color = args.get::<bool>("color").unwrap_or(false);
    if args.orphans.len() < 1 {
        panic!("String not provided");
    }
    let s = &args.orphans[0];

    let frames = generate_frames(s, color);
    let stdout = io::stdout();
    let mut encoder = GifEncoder::new(stdout);
    _ = encoder.encode_frames(frames);
}
