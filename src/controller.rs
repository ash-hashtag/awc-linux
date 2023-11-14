use std::{
    fs::OpenOptions,
    io::{Read, Write},
    sync::atomic::{AtomicBool, AtomicIsize, Ordering},
    thread,
    time::{Duration, SystemTime},
};

const ACPI_CALL_FPATH: &'static str = "/proc/acpi/call";

const RESET: &'static str = "\x1b[0m";
const BOLD: &'static str = "\x1b[1m";
const RED: &'static str = "\x1b[31m";
const GREEN: &'static str = "\x1b[32m";
const YELLOW: &'static str = "\x1b[33m";
const BLUE: &'static str = "\x1b[34m";
const CYAN: &'static str = "\x1b[36m";

#[derive(Debug)]
struct AlienDevInfo {
    fan_id: u8,
    sen_id: u8,
    name: &'static str,
}

const ALIEN_DEVICES: [AlienDevInfo; 2] = [
    AlienDevInfo {
        fan_id: 50,
        sen_id: 1,
        name: "CPU",
    },
    AlienDevInfo {
        fan_id: 51,
        sen_id: 6,
        name: "GPU",
    },
];

#[derive(Debug)]
pub struct AlienDevGraphInfo {
    dev: &'static AlienDevInfo,
    graph: Vec<CoOrdinates>,
    last_fan_boost: u8,
    last_fan_rpm_recorded: LastFanRPMRecorded,
}

#[derive(Debug)]
pub struct LastFanRPMRecorded {
    rpm: i64,
    ts: SystemTime,
}

pub struct Controller {
    alien_dev_graph_infos: [AlienDevGraphInfo; 2],
    power_mode: u8,
}

impl Controller {
    pub fn new(graph_path: &str) -> Self {
        let alien_dev_graph_infos = load_graph(graph_path);
        let power_mode = get_power_mode() as u8;
        dbg!(&alien_dev_graph_infos);

        Self {
            power_mode,
            alien_dev_graph_infos,
        }
    }

    // pub fn update(&mut self) {
    //     for info in &mut self.alien_dev_graph_infos {
    //         let temp = get_temp(info.dev.sen_id) as u8;
    //         print!(
    //             "{} Sensor {BOLD}#{}{RESET} Temp: {YELLOW}{}{RESET}\n",
    //             info.dev.name, info.dev.sen_id, temp
    //         );
    //         let rpm = get_fan_rpm(info.dev.fan_id);
    //         if info.last_fan_rpm_recorded.ts.elapsed().unwrap().as_secs() > 0 {}
    //         let boost = get_boost_from_temp(temp, &info.graph);
    //         if boost != info.last_fan_boost {
    //             let result = set_fan_boost(info.dev.fan_id, boost);
    //             print!(
    //                 "Fan {BOLD}#{}{RESET} Boost: {YELLOW}{}{RESET}/255 RPM: {GREEN}{}{RESET} Result: {}\n",
    //                 info.dev.fan_id, boost,rpm, result
    //             );
    //             info.last_fan_boost = boost;
    //         } else {
    //             let rpm = get_fan_rpm(info.dev.fan_id);
    //             print!(
    //                 "Fan {BOLD}#{}{RESET} Boost: {YELLOW}{}{RESET}/255 RPM: {GREEN}{}{RESET}\n",
    //                 info.dev.fan_id, info.last_fan_boost, rpm
    //             );
    //         }
    //     }
    // }

    pub fn watch(&mut self, update_interval_in_seconds: u64, exit_sig: &AtomicIsize) {
        let milli_sec_dur = Duration::from_millis(200);
        loop {
            if self.power_mode == 0 {
                let current_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                print!("{CYAN}{}{RESET}\n", current_time);
                print!("Power Mode: {BOLD}{}{RESET}\n", self.power_mode);
                for info in &mut self.alien_dev_graph_infos {
                    {
                        // Some bug fix where fans stuck at the same rpm and won't change
                        let rpm = get_fan_rpm(info.dev.fan_id);
                        if !(rpm == 0 && info.last_fan_boost == 0)
                            && info.last_fan_rpm_recorded.rpm == rpm
                            && info.last_fan_rpm_recorded.ts.elapsed().unwrap().as_secs()
                                > update_interval_in_seconds * 3
                        {
                            let result = set_fan_boost(info.dev.fan_id, 0);
                            print!("Fan {BOLD}#{}{RESET} Boost: {YELLOW}0{RESET}/255 RPM: {GREEN}{}{RESET} Result: {}\n",info.dev.fan_id, rpm, result);
                            thread::sleep(milli_sec_dur);
                        }
                    }
                    let rpm = get_fan_rpm(info.dev.fan_id);
                    if rpm != info.last_fan_rpm_recorded.rpm {
                        info.last_fan_rpm_recorded = LastFanRPMRecorded {
                            rpm,
                            ts: SystemTime::now(),
                        };
                    }
                    let temp = get_temp(info.dev.sen_id) as u8;
                    print!(
                        "{} Sensor {BOLD}#{}{RESET} Temp: {YELLOW}{}{RESET}\n",
                        info.dev.name, info.dev.sen_id, temp
                    );
                    let boost = get_boost_from_temp(temp, &info.graph);
                    if boost != info.last_fan_boost {
                        let result = set_fan_boost(info.dev.fan_id, boost);
                        print!("Fan {BOLD}#{}{RESET} Boost: {YELLOW}{}{RESET}/255 RPM: {GREEN}{}{RESET} Result: {}\n",info.dev.fan_id, boost,rpm, result);
                        info.last_fan_boost = boost;
                    } else {
                        let rpm = get_fan_rpm(info.dev.fan_id);
                        print!("Fan {BOLD}#{}{RESET} Boost: {YELLOW}{}{RESET}/255 RPM: {GREEN}{}{RESET}\n",info.dev.fan_id, info.last_fan_boost, rpm);
                    }
                }
            }
            for i in 0..(update_interval_in_seconds * 5) {
                let sig_val = exit_sig.load(Ordering::SeqCst);
                if sig_val != 0 {
                    exit_sig.store(0, Ordering::SeqCst);
                    match sig_val {
                        -1 => {
                            if self.power_mode == 0 {
                                set_both_fan_boosts(0);
                            } else {
                                self.toggle_mode();
                            }
                            print!("Watching Stopped Gracefully\n");
                            return;
                        }
                        1 => {
                            self.toggle_mode();
                        }
                        2 => show_all_info(),
                        3 => {
                            if self.power_mode == 0 {
                                set_both_fan_boosts(0);
                            }
                        }
                        4 => {
                            break;
                        }
                        _ => {}
                    }
                }

                thread::sleep(milli_sec_dur);
            }
        }
    }

    pub fn toggle_mode(&mut self) {
        self.power_mode = toggle_power_mode() as u8;
    }
}

pub fn set_both_fan_boosts(value: u8) {
    for dev in &ALIEN_DEVICES {
        print!(
            "Fan #{BOLD}{}{RESET}: {YELLOW}{}{RESET}/255 result: {}\n",
            dev.fan_id,
            value,
            set_fan_boost(dev.fan_id, value)
        );
    }
}
pub fn show_temps() {
    for dev in &ALIEN_DEVICES {
        print!(
            "Sensor {} {BOLD}#{}{RESET}: {YELLOW}{}{RESET}\n",
            dev.name,
            dev.sen_id,
            get_temp(dev.sen_id)
        );
    }
}
pub fn show_fan_boosts() {
    for dev in &ALIEN_DEVICES {
        print!(
            "Fan {BOLD}#{}{RESET}:\n boost: {YELLOW}{}{RESET}/255, rpm: {GREEN}{}{RESET}\n",
            dev.fan_id,
            get_fan_boost(dev.fan_id),
            get_fan_rpm(dev.fan_id)
        );
    }
}

fn get_fan_boost(fan_id: u8) -> u8 {
    run_main_command(0x14, 0xc, fan_id, 0) as u8
}

fn set_fan_boost(fan_id: u8, value: u8) -> i64 {
    run_main_command(0x15, 2, fan_id, value)
}

fn get_fan_rpm(fan_id: u8) -> i64 {
    run_main_command(0x14, 5, fan_id, 0)
}

fn get_temp(sen_id: u8) -> i64 {
    run_main_command(0x14, 4, sen_id, 0)
}

#[derive(Debug)]
pub struct CoOrdinates {
    temp: u8,
    fan_boost: u8,
}

pub fn load_graph(file_path: &str) -> [AlienDevGraphInfo; 2] {
    let mut buf = String::with_capacity(1024);
    OpenOptions::new()
        .read(true)
        .open(file_path)
        .unwrap()
        .read_to_string(&mut buf)
        .unwrap();

    let mut lines = buf.lines();
    let line0 = lines.next().unwrap();
    let line1 = lines.next().unwrap();

    let now = SystemTime::now();

    return [
        AlienDevGraphInfo {
            dev: &ALIEN_DEVICES[0],
            graph: line_to_coords(line0),
            last_fan_boost: get_fan_boost(ALIEN_DEVICES[0].fan_id),
            last_fan_rpm_recorded: LastFanRPMRecorded {
                rpm: get_fan_rpm(ALIEN_DEVICES[0].fan_id),
                ts: now,
            },
        },
        AlienDevGraphInfo {
            dev: &ALIEN_DEVICES[1],
            graph: line_to_coords(line1),
            last_fan_boost: get_fan_boost(ALIEN_DEVICES[1].fan_id),
            last_fan_rpm_recorded: LastFanRPMRecorded {
                rpm: get_fan_rpm(ALIEN_DEVICES[1].fan_id),
                ts: now,
            },
        },
    ];
}

fn line_to_coords(line: &str) -> Vec<CoOrdinates> {
    let mut v = Vec::<CoOrdinates>::with_capacity(32);
    let mut last_temp = 0u8;
    let mut first_time = true;
    for coord in line.split(',') {
        let c = coord.trim();
        let mut s = c[1..c.len() - 1].split_whitespace();
        let temp = s.next().unwrap().parse().unwrap();
        if !first_time && last_temp >= temp {
            panic!("Temps must be ascending order!!!");
        }
        let fan_boost = s.next().unwrap().parse().unwrap();
        v.push(CoOrdinates { temp, fan_boost });
        last_temp = temp;
        if first_time {
            first_time = false;
        }
    }
    return v;
}

fn get_boost_from_temp(temp: u8, coords: &Vec<CoOrdinates>) -> u8 {
    for i in 0..coords.len() - 1 {
        let a = &coords[i];
        let b = &coords[i + 1];
        if temp >= a.temp && temp < b.temp {
            let boost =
                a.fan_boost + (temp - a.temp) * ((b.fan_boost - a.fan_boost) / (b.temp - a.temp));
            return boost;
        }
    }
    return coords.last().unwrap().fan_boost;
}

// fn probe_info() -> Probes {
//     let sys_id = run_main_command(0x1a, 2, 2, 0);
//     print!("Probe Allowed: {}\n", run_main_command(0x14, 2, 0, 0));
//     print!("System Id: {}\n", sys_id);

//     let mut f_index = 0;
//     let mut func_id = 0;
//     let mut fans = Vec::<AlienFanInfo>::with_capacity(8);
//     let mut sensors = Vec::<AlienSenInfo>::with_capacity(8);
//     let mut powers = Vec::<u8>::with_capacity(8);
//     loop {
//         let func_id = run_main_command(0x14, 3, f_index, 0);
//         if func_id < 0x100 && func_id > 0 || func_id > 0x130 {
//             fans.push(AlienFanInfo {
//                 id: func_id as u8 & 0xff,
//                 fan_type: 0xff,
//             });
//             f_index += 1;
//         } else {
//             break;
//         }
//     }

//     print!(
//         "{f_index} Fans Detected, Last Reply {func_id}:\n{:?}\n",
//         fans
//     );

//     let first_sensor_index = f_index;
//     loop {
//         let s_index = (f_index - first_sensor_index) as usize;
//         let res = run_main_command(0x14, 4, func_id as u8, 0);
//         if res > 0 {
//             let name = format!("Sensor #{s_index}");
//             sensors.push(AlienSenInfo {
//                 id: func_id as u8,
//                 sen_type: 1,
//             });
//             f_index += 1;
//         }
//         func_id = run_main_command(0x14, 3, f_index, 0);
//         if !(func_id > 0x100 && func_id < 0x1A0) {
//             break;
//         }
//     }
//     print!(
//         "{} Sensors Detected, Last Reply {func_id}:\n {:?}\n",
//         sensors.len(),
//         sensors
//     );
//     if (func_id > 0) {
//         loop {
//             powers.push(func_id as u8 & 0xff);
//             f_index += 1;
//             func_id = run_main_command(0x14, 3, f_index, 0);
//             if func_id <= 0 {
//                 break;
//             }
//         }
//         print!(
//             "{} Power Modes Detected, Last Reply {func_id}:\n {:?}\n",
//             powers.len(),
//             powers
//         );
//     }

//     for j in 0..fans.len() {
//         let index = run_main_command(0x13, 2, fans[j].id, 0);
//         let sensor = AlienSenInfo {
//             id: index as u8,
//             sen_type: 1,
//         };
//         for i in 0..sensors.len() {
//             if sensors[i].id == sensor.id && sensors[i].sen_type == sensor.sen_type {
//                 fans[j].fan_type = i as u8;
//                 break;
//             }
//         }
//     }
//     print!("Fans: {:?}\n\n\n\n\n", fans);
//     return Probes {
//         sensors,
//         fans,
//         powers,
//         sys_id,
//     };
// }

pub fn show_all_info() {
    let mode = get_power_mode();
    print!("Power Mode: {mode}\n");
    for dev in &ALIEN_DEVICES {
        let temp = get_temp(dev.sen_id);
        let rpm = get_fan_rpm(dev.fan_id);
        let boost = get_fan_boost(dev.fan_id);

        print!("{}: \n", dev.name);
        print!(
            " Sensor {BOLD}#{}{RESET} Temp: {YELLOW}{}{RESET}\n",
            dev.sen_id, temp
        );
        print!(
            " Fan {BOLD}#{}{RESET} boost: {YELLOW}{}{RESET}/255 rpm: {GREEN}{}{RESET}\n",
            dev.fan_id, boost, rpm
        );
    }
}

pub fn toggle_power_mode() -> u8 {
    if get_power_mode() == 0 {
        print!("{BOLD}{GREEN}Enabled Power Mode\n{RESET}");
        set_power_mode(0xab);
        0xab
    } else {
        print!("{BOLD}Disabled Power Mode\n{RESET}");
        set_power_mode(0);
        0
    }
}

fn get_power_mode() -> i64 {
    run_main_command(0x14, 0xb, 0, 0)
}

fn enable_gmode() -> i64 {
    set_power_mode(0xab)
}

fn disable_gmode() -> i64 {
    set_power_mode(0)
}

fn set_power_mode(mode: u8) -> i64 {
    run_main_command(0x15, 1, mode, 0)
}

fn toggle_gmode(mode: u8) -> i64 {
    run_main_command(0x25, 1, mode, 0)
}

fn run_command(cmd: &str) -> String {
    let mut f = OpenOptions::new()
        .write(true)
        .read(true)
        .truncate(true)
        .open(ACPI_CALL_FPATH)
        .unwrap();
    f.write(cmd.as_bytes()).unwrap();
    let mut s = String::with_capacity(32);
    f.read_to_string(&mut s);
    s.pop();
    s
}

fn run_main_command(cmd: u8, sub: u8, arg0: u8, arg1: u8) -> i64 {
    let s = format!("\\_SB.AMW3.WMAX 0 {cmd} {{ {sub}, {arg0}, {arg1}, 0 }}");
    let result = run_command(&s);
    let res = i64::from_str_radix(&result[2..], 16).unwrap();
    res
}
