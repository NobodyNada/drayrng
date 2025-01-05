use std::f32::consts::PI;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smrng::Rng;

#[derive(Clone)]
struct Draygon {
    x: i16,
    y: i16,
    y_angle: u8,
    goop_counter: u8,
    goop_timer: u8,
    global_timer: u8,
    left: bool,
    samus_in_range: bool,
}

struct Samus {
    x: i16,
    y: i16,
}

fn cosmul(x: u8, theta: u8) -> i32 {
    let theta = theta.wrapping_add(0x40);
    let sin = (((theta & 0x7F) as f32 / 0x80 as f32 * PI).sin() * 256.) as u8;
    let mul = x as u16 * sin as u16;

    let mut int = mul >> 8;
    let mut frac = (mul & 0xFF) << 8;

    if theta >= 0x80 {
        int = -(int as i16) as u16;
        frac = -(frac as i16) as u16;
    }

    ((int as u32) << 16 | (frac as u32)) as i32
}

//fn cosmul_naive(x: u8, theta: u8) -> i32 {
//(x as f32 * (theta as f32 / 0x80 as f32 * PI).cos() * 65536.) as i32
//}

impl Draygon {
    fn simulate_goop(mut rng: Rng, global_timer: u8, left: bool, samus: &Samus) -> bool {
        let x = if left { 0x250 } else { 0xFFB0u16 as i16 };
        let mut state = Draygon {
            x,
            y: 0x180,
            y_angle: 0,
            goop_counter: 0x10,
            goop_timer: 0,
            global_timer,
            left,
            samus_in_range: false,
        };

        loop {
            if let Some(gooped) = state.step(&mut rng, samus) {
                return gooped;
            }
            rng.frame_advance();
        }
    }

    fn step(&mut self, rng: &mut Rng, samus: &Samus) -> Option<bool> {
        if self.global_timer & 0x3F == 0 && (!self.left || !self.samus_in_range) {
            // turret firing
            rng.roll();
        }

        self.y = 0x180 + (cosmul(0x20, self.y_angle) >> 16) as i16;
        if !self.samus_in_range {
            if self.x.abs_diff(samus.x) < 0xD0 {
                self.samus_in_range = true;
            } else {
                self.x += if self.left { -1 } else { 1 };
                self.y_angle = self.y_angle.wrapping_add(1);
            }
        } else {
            if rng.read() & 0xF == 0 {
                self.goop_timer = 7;
            }

            self.x += if self.left { -1 } else { 1 };
            self.y_angle = self.y_angle.wrapping_add(1);
        }
        if self.goop_timer != 0 {
            self.goop_timer -= 1;
            if self.goop_timer == 0 {
                self.goop_counter -= 1;
                if self.goop_counter == 0 {
                    return Some(false);
                }
                if self.fire_goop(rng, samus) {
                    return Some(true);
                }
            }
        }

        if self.global_timer & 0x7F == 0 {
            // bubble blubbing
            rng.roll();
        }
        self.global_timer = self.global_timer.wrapping_add(1);

        if (!self.left && self.x >= 0x2A0) || (self.left && self.x < -0x50) {
            Some(false)
        } else {
            None
        }
    }

    fn fire_goop(&mut self, rng: &mut Rng, samus: &Samus) -> bool {
        let x = if self.left {
            self.x - 0x1C
        } else {
            self.x + 0x18
        };
        let y = self.y - 0x10;

        /*println!(
            "{:#04x}:    {:#05x}, {:#05x}",
            self.global_timer, self.x, self.y
        );*/
        let mut x = (x as i32) << 16;
        let mut y = (y as i32) << 16;

        let angle = if self.left { 0xA0 } else { 0xE0 };
        let angle = angle + (rng.roll() & 0x3F) - 0x20;
        //println!("{:#x}", angle);
        let angle = (angle as f32) * PI / 0x80 as f32;

        let speed = 2;

        let vx = speed * (angle.cos() * 65535.) as i32;
        let vy = speed * (-angle.sin() * 65535.) as i32;

        while (0..=512).contains(&(x >> 16)) && (0..=512).contains(&(y >> 16)) {
            x += vx;
            y += vy;

            if samus.x.abs_diff((x >> 16) as i16) < 0x10
                && samus.y.abs_diff((y >> 16) as i16) < 0x14
            {
                return true;
            }
        }

        false
    }
}

fn main() {
    //2809, 0xC93E, *0xEF47*, 0xAD74, 0x6455
    let seeds = std::mem::take(&mut Rng::RESET.analyze().loops[0].seeds);

    /*for seed in seeds.iter().copied() {
        let rng = Rng::RESET.with_seed(seed);
        let y = 0x01C9;
        let x = 0x00b9;

        if !Draygon::simulate_goop(rng, 0x1f, true, &Samus { x, y }) {
            println!("{:04x}", seed);
        }
    }*/

    /*let rng = Rng::RESET.with_seed(0x0000);
    let y = 0x01C9;
    let x = 0x00b9;
    println!(
        "{}",
        Draygon::simulate_goop(rng, 0x88, false, &Samus { x, y })
    );*/

    let global_timer_range = 0x80;
    let total_seeds = seeds.len() * global_timer_range as usize;

    for left in [true, false] {
        println!("{}:", if left { "left" } else { "right" });
        let y = 0x01C9;
        for x in 0x45..=0x019B {
            let samus = Samus { x, y };
            let num_seeds = seeds
                .par_iter()
                .flat_map_iter(|&seed| {
                    (0..global_timer_range).map(move |global_timer| (seed, global_timer))
                })
                .filter(|&(seed, global_timer)| {
                    Draygon::simulate_goop(Rng::RESET.with_seed(seed), global_timer, left, &samus)
                })
                .count();

            let percent = (num_seeds as f32 / total_seeds as f32 * 10000.).round() / 100.;
            println!(
                "    {:#05x}: {:02.02}% ({} / {})",
                x, percent, num_seeds, total_seeds
            );
        }
    }
}
