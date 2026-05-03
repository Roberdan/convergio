
use colored::*;
use rand::Rng;
use std::{thread, time};

fn gradient_char(c: char, i: usize, len: usize) -> colored::ColoredString {
    let ratio = i as f32 / len as f32;
    let r = (255.0 * (1.0 - ratio)) as u8;
    let g = (200.0 * ratio) as u8;
    let b = 255;
    c.to_string().truecolor(r, g, b)
}

fn render_gradient(text: &str) {
    let len = text.len();
    for (i, c) in text.chars().enumerate() {
        print!("{}", gradient_char(c, i, len));
    }
    println!();
}

fn glitch(text: &str) -> String {
    let mut rng = rand::thread_rng();
    text.chars()
        .map(|c| {
            if rng.gen_bool(0.1) {
                match c {
                    'O' => '0',
                    'E' => '_',
                    _ => c,
                }
            } else {
                c
            }
        })
        .collect()
}

fn main() {
    let base = "CONVERGIO";

    for _ in 0..3 {
        print!("\x1B[2J\x1B[1;1H");
        println!("{}", glitch(base).truecolor(255, 0, 180));
        thread::sleep(time::Duration::from_millis(80));
    }

    println!("{}", "> booting convergio kernel...".truecolor(0,200,255));
    thread::sleep(time::Duration::from_millis(200));
    println!("{}", "> syncing nodes...".truecolor(0,200,255));
    thread::sleep(time::Duration::from_millis(200));
    println!("{}", "> convergence achieved.".truecolor(255,0,180));

    println!();
    render_gradient(base);
}
