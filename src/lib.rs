//! Giddy Ganja: the game rules, levels and AI. This library has no graphics or audio dependencies,
//! so all of it is covered by headless tests; the `giddy-ganja` binary adds the window and sound.

// Grid code indexes `[y][x]` arrays by coordinate on purpose; iterator rewrites would be less clear.
#![allow(clippy::needless_range_loop)]

pub mod ai;
pub mod bot;
pub mod game;
pub mod grid;
pub mod levels;
pub mod maze;
pub mod rng;
pub mod storage;
