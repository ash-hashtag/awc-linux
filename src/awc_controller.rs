use std::{
    sync::{
        atomic::{AtomicIsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, SystemTime},
};

use crate::{
    config::{AwcConfig, DeviceInfo, GraphType},
    controller::{
        get_boost_from_temp_linear, get_boost_from_temp_step, get_fan_boost, get_fan_rpm,
        get_power_mode, get_temp, set_both_fan_boosts, set_fan_boost, show_all_info,
        toggle_power_mode, LastFanRPMRecorded,
    },
};

const ACPI_CALL_FPATH: &'static str = "/proc/acpi/call";

const RESET: &'static str = "\x1b[0m";
const BOLD: &'static str = "\x1b[1m";
const RED: &'static str = "\x1b[31m";
const GREEN: &'static str = "\x1b[32m";
const YELLOW: &'static str = "\x1b[33m";
const BLUE: &'static str = "\x1b[34m";
const CYAN: &'static str = "\x1b[36m";

pub struct AwcController {
    power_mode: u8,
    cpu: AwcDeviceInfo,
    gpu: AwcDeviceInfo,
    interval: u64,
}

pub struct AwcDeviceInfo {
    last_fan_boost: u8,
    last_fan_rpm: LastFanRPMRecorded,
    device: DeviceInfo,
}

impl AwcDeviceInfo {
    pub fn new(device: DeviceInfo) -> Self {
        let last_fan_boost = get_fan_boost(device.fan);
        let last_fan_rpm = LastFanRPMRecorded {
            ts: SystemTime::now(),
            rpm: get_fan_rpm(device.fan),
        };

        Self {
            last_fan_boost,
            last_fan_rpm,
            device,
        }
    }
}

impl AwcController {
    pub fn new(config: AwcConfig) -> Self {
        let power_mode = get_power_mode() as u8;
        let cpu = AwcDeviceInfo::new(config.cpu);
        let gpu = AwcDeviceInfo::new(config.gpu);
        let interval = config.interval;

        AwcController {
            power_mode,
            cpu,
            gpu,
            interval,
        }
    }

    pub fn watch(&mut self, sig: Arc<AtomicIsize>) {
        if self.power_mode != 0 {
            self.power_mode = toggle_power_mode();
        }

        loop {
            if self.power_mode != 0 {
                continue;
            }

            let current_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            print!("{CYAN}{}{RESET}\n", current_time);
            print!("Power Mode: {BOLD}{}{RESET}\n", self.power_mode);

            update_boosts(&mut self.cpu);
            update_boosts(&mut self.gpu);

            let start = SystemTime::now();

            while start.elapsed().unwrap().as_secs() > self.interval {
                let sig_val = sig.load(Ordering::SeqCst);
                if sig_val != 0 {
                    sig.store(0, Ordering::SeqCst);
                    match sig_val {
                        -1 => {
                            if self.power_mode == 0 {
                                self.set_both_fan_boosts(0);
                            } else {
                                self.toggle_mode();
                            }
                            return;
                        }
                        1 => {
                            self.toggle_mode();
                        }
                        2 => self.show_stats(),
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

                thread::sleep(Duration::from_millis(500));
            }
        }
    }

    pub fn toggle_mode(&mut self) {
        self.power_mode = toggle_power_mode();
    }

    pub fn show_stats(&self) {
        let mode = self.power_mode;
        print!("Power Mode: {mode}\n");
        show_device_stats(&self.cpu);
        show_device_stats(&self.gpu);
    }
}

pub fn show_device_stats(device: &AwcDeviceInfo) {
    let temp = get_temp(device.device.sensor);
    let rpm = get_fan_rpm(device.device.fan);
    let boost = get_fan_boost(device.device.fan);

    print!(
        " Sensor {BOLD}#{}{RESET} Temp: {YELLOW}{}{RESET}\n",
        device.device.sensor, temp
    );
    print!(
        " Fan {BOLD}#{}{RESET} boost: {YELLOW}{}{RESET}/255 rpm: {GREEN}{}{RESET}\n",
        device.device.fan, boost, rpm
    );
}

pub fn update_boosts(info: &mut AwcDeviceInfo) {
    let rpm = get_fan_rpm(info.device.fan);
    if !(rpm == 0 && info.last_fan_boost == 0) {
        if info.last_fan_rpm.rpm == rpm && info.last_fan_rpm.ts.elapsed().unwrap().as_secs() > 90 {
            let result = set_fan_boost(info.device.fan, 0);
            print!("Fan {BOLD}#{}{RESET} Boost: {YELLOW}0{RESET}/255 RPM: {CYAN}{}{RESET} Result: {}\n",info.device.fan, rpm, result);
            thread::sleep(Duration::from_millis(200));
        }
    }
    let rpm = get_fan_rpm(info.device.fan);
    if rpm != info.last_fan_rpm.rpm {
        info.last_fan_rpm = LastFanRPMRecorded {
            rpm,
            ts: SystemTime::now(),
        };
    }
    let temp = get_temp(info.device.sensor) as u8;
    print!(
        "Sensor {BOLD}#{}{RESET} Temp: {YELLOW}{}{RESET}\n",
        info.device.sensor, temp
    );
    let boost = match info.device.graph_type {
        GraphType::Linear => get_boost_from_temp_linear(temp, &info.device.graph),
        GraphType::Step => get_boost_from_temp_step(temp, &info.device.graph),
    };
    if boost != info.last_fan_boost {
        let result = set_fan_boost(info.device.fan, boost);
        print!(
            "Fan {BOLD}#{}{RESET} Boost: {YELLOW}{}{RESET}/255 RPM: {GREEN}{}{RESET} Result: {}\n",
            info.device.fan, boost, rpm, result
        );
        info.last_fan_boost = boost;
    } else {
        // let rpm = get_fan_rpm(info.dev.fan_id);
        print!(
            "Fan {BOLD}#{}{RESET} Boost: {YELLOW}{}{RESET}/255 RPM: {GREEN}{}{RESET}\n",
            info.device.fan, info.last_fan_boost, rpm
        );
    }
}
