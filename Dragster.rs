// Calculates optimal times and inputs for Dragster.

use std::io::Write;

fn main() {
  println!("|| tach\\offset ||     0-1     ||     2-3     ||     4-5     ||     6-7     ||     8-9     ||    10-11    ||    12-13    ||    14-15    ||");
  for tach in 0..32 {
    print!("|      {:2}      ", tach);
    ::std::io::stdout().flush().ok();
    for offset in 0..8 {
      let (first_frame, win_frame, distance, _) = find_optimal_inputs(offset, tach, true);
      let first_time = 0.0334 * first_frame as f64 - 0.005;
      let win_time = 0.0334 * win_frame as f64 - 0.005;
      print!("| {:4.2}/{:4.2}/{:02x} ", first_time, win_time, distance - 0x6100);
      ::std::io::stdout().flush().ok();
    }
    println!("|");
  }
  // let (_, _, _, p1_inputs) = find_optimal_inputs(5, 27, true);
  // let (_, _, _, p2_inputs) = find_optimal_inputs(6, 27, false);
  // for input in interleave_inputs(p1_inputs, p2_inputs) {
  //   println!("{}", input);
  // }
}

#[derive(Debug)]
pub struct State {
  global_frame_counter_halfed: u8,
  player_gear: u8,
  shift: bool,
  speed: u8,
  tachometer: u8,
}
impl State {
  fn to_index(&self) -> usize {
    return self.global_frame_counter_halfed as usize
        + self.player_gear as usize * 8
        + if self.shift { 8 * 5 } else { 0 }
        + self.speed as usize * 8 * 5 * 2
        + self.tachometer as usize * 8 * 5 * 2 * 256;
  }
  fn from_index(index: usize) -> State {
    return State {
      global_frame_counter_halfed: (index % 8) as u8,
      player_gear: ((index / 8) % 5) as u8,
      shift: (index / 8 / 5) % 2 != 0,
      speed: ((index / 8 / 5 / 2) % 256) as u8,
      tachometer: ((index / 8 / 5 / 2 / 256) % 32) as u8,
    }
  } 
}

fn interleave_inputs(p1_inputs: Vec<String>, p2_inputs: Vec<String>) -> Vec<String> {
  let mut inputs: Vec<String> = Vec::new();
  let len = std::cmp::max(p1_inputs.len(), p2_inputs.len());
  for i in 0..len {
    inputs.push(format!("|.....|{}|.....|", if p1_inputs.len() > i { &p1_inputs[i] } else { "....." }));
    inputs.push(format!("|.....|.....|{}|", if p2_inputs.len() > i { &p2_inputs[i] } else { "....." }));
  }
  inputs
}

fn find_optimal_inputs(frame_offset: u8, tach: u8, find_fewest_input_frames: bool) -> (usize, usize, i16, Vec<String>) {
  // create DP arrays
  let mut max_distance: Vec<i16> = Vec::with_capacity(8 * 5 * 2 * 256 * 32 * 200);
  max_distance.resize(8 * 5 * 2 * 256 * 32 * 200, -1);
  let mut max_distance_prev: Vec<usize> = Vec::with_capacity(8 * 5 * 2 * 256 * 32 * 200);
  max_distance_prev.resize(8 * 5 * 2 * 256 * 32 * 200, 0);
  let mut max_distance_input: Vec<u8> = Vec::with_capacity(8 * 5 * 2 * 256 * 32 * 200);
  max_distance_input.resize(8 * 5 * 2 * 256 * 32 * 200, 0);

  { // Initialize DP array
    let mut s = State::from_index(0);
    s.shift = true;
    s.tachometer = tach;
    s.global_frame_counter_halfed = frame_offset;
    max_distance[s.to_index()] = 0;
  }

  // fill DP arrays
  let mut frame: usize = 0;
  let mut max_dist = 0;
  let mut max_dist_index = 0;
  let mut first_win_frame = 200;
  let mut min_win_frame = 0;
  let mut max_win_distance = 0;
  let mut win_index = 0;
  loop {
    for input in 0..4 {
      for i in 0..(8 * 5 * 2 * 256 * 32) {
        if max_distance[frame * 8 * 5 * 2 * 256 * 32 + i] >= 0 {
          if find_fewest_input_frames {
            let (win, win_frames, win_distance) = can_win_without_any_inputs(State::from_index(i), frame, max_distance[frame * 8 * 5 * 2 * 256 * 32 + i]);
            if win && (first_win_frame > frame || (first_win_frame == frame && min_win_frame > win_frames) || (first_win_frame == frame && min_win_frame == win_frames && max_win_distance < win_distance)) {
              first_win_frame = frame;
              min_win_frame = win_frames;
              max_win_distance = win_distance;
              win_index = frame * 8 * 5 * 2 * 256 * 32 + i;
            }
          }
          let mut s = State::from_index(i);
          let new_distance = max_distance[frame * 8 * 5 * 2 * 256 * 32 + i] + s.speed as i16;
          let gas = input & 1 != 0;
          let shift = input & 2 != 0;
          if emulate_frame(&mut s, gas, shift) {
            let new_index = (frame + 1) * 8 * 5 * 2 * 256 * 32 + s.to_index();
            if new_distance > max_distance[new_index] {
              max_distance[new_index] = new_distance;
              max_distance_prev[new_index] = frame * 8 * 5 * 2 * 256 * 32 + i;
              max_distance_input[new_index] = input;
            }
            if new_distance > max_dist {
              max_dist = new_distance;
              max_dist_index = new_index;
            }
          }
        }
      }
    }
    if first_win_frame < 200 { break; }
    frame += 1;
    if max_dist >= 0x6100 { break; }
  }
  if !find_fewest_input_frames {
    first_win_frame = frame;
    min_win_frame = frame;
    max_win_distance = max_dist;
    win_index = max_dist_index;
  }

  let mut inputs: Vec<String> = Vec::new();
  loop {
    if win_index < 8 * 5 * 2 * 256 * 32 {
      inputs.reverse();
      return (first_win_frame, min_win_frame, max_win_distance, inputs);
    }
    let input = max_distance_input[win_index];
    win_index = max_distance_prev[win_index];
    inputs.push((format!("..{}.{}", if input & 2 != 0 { "L" } else { "." }, if input & 1 != 0 { "B" } else { "." })));
  }
}

fn can_win_without_any_inputs(mut s: State, mut frame: usize, mut distance: i16) -> (bool, usize, i16) {
  loop {
    if distance >= 0x6100 { return (true, frame, distance); }
    if s.speed == 0 { return (false, 0, 0); }
    distance += s.speed as i16;
    if !emulate_frame(&mut s, false, false) { return (false, 0, 0); }
    frame += 1;
  }
}

fn emulate_frame(s: &mut State, gas: bool, shift: bool) -> bool {
  s.global_frame_counter_halfed = (s.global_frame_counter_halfed + 1) % 8;

  // ProcessActivePlayer
  // distance += speed occurs here

  let y: usize = if s.shift { 0 } else { s.player_gear as usize };
  if s.global_frame_counter_halfed % (1 << y.saturating_sub(1)) == 0 {
    if gas {
      s.tachometer += if y == 0 { 3 } else { 1 };
    } else {
      s.tachometer = s.tachometer.saturating_sub(if y == 0 { 3 } else { 1 });
    }
    if s.tachometer >= 0x20 {
      return false; // engine blown
    }
  }
  if y > 0 {
    let speed_ceiling = (s.tachometer << (y - 1)) + if y >= 2 && s.tachometer >= 20 { 1 << (y - 2) } else { 0 };
    if speed_ceiling > s.speed {
      if speed_ceiling - s.speed >= 16 {
        s.tachometer -= 1;
      }
      s.speed += 2;
    } else if speed_ceiling < s.speed {
      s.speed = s.speed.saturating_sub(1);
    }
  }

  if !shift && s.shift {
    s.player_gear += 1;
    if s.player_gear > 4 { s.player_gear = 4; }
  }
  s.shift = shift;

  return true;
}
