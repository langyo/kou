//! Print the LLM-friendly structured reading of a sample screen.
//!
//! Run: `cargo run --example read_demo`

use kou::{ReadStyle, Screen, read};

fn main() {
    // Each data row is laid out so the state word starts at column 11 (1-based)
    // = 0-based col 10, matching the "State" header.
    let mut s = Screen::new(40, 6);
    s.feed(b"\x1b[1m  Service Status  \x1b[0m\n");
    s.feed(b"Name      State      Notes\n");
    s.feed(b"alpha     OK         started\n");
    s.feed(b"bravo     WARN       standby\n");
    s.feed(b"charlie   ERR        crashed\n");
    // Recolour the state word in place (1-based CUP, column 11).
    s.feed(b"\x1b[3;11H\x1b[32mOK\x1b[0m");
    s.feed(b"\x1b[4;11H\x1b[33mWARN\x1b[0m");
    s.feed(b"\x1b[5;11H\x1b[31mERR\x1b[0m");

    println!(
        "===== STRUCTURED =====\n{}\n",
        read(&s, ReadStyle::Structured)
    );
    println!("===== BOXED =====\n{}\n", read(&s, ReadStyle::Boxed));
    println!("===== RAW =====\n{}", read(&s, ReadStyle::Raw));
}
