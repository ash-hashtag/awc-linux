#![allow(unused)]

mod controller;

use std::{
    fs::{self, File, OpenOptions},
    io::{stdin, Read, Write},
    os::unix::net::UnixListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use clap::{CommandFactory, Parser, Subcommand};
use controller::*;

const SOCKET_PATH: &'static str = "/tmp/awcc";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct CmdArgs {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Watch {
        #[arg(default_value_t = 30, short, long)]
        interval: u64,

        #[arg(short, long)]
        path: String,
    },

    Info,

    Temps,

    Mode,

    Fans {
        #[arg(short, long)]
        boost: Option<u8>,
    },
}

fn main() {
    let args = CmdArgs::parse();
    handle_args(args);
}

fn handle_args(args: CmdArgs) {
    match args.commands {
        Commands::Watch { interval, path } => {
            let exit_signal = Arc::new(AtomicBool::new(false));
            let ctrc_exit_sig = exit_signal.clone();
            ctrlc::set_handler(move || {
                ctrc_exit_sig.store(true, Ordering::SeqCst);
            })
            .unwrap();
            print!("Update Interval: {interval} seconds\n");
            let t = thread::spawn(move || {
                let mut controller = Controller::new(&path);
                controller.watch(interval, &exit_signal);
            });
            t.join();
        }
        Commands::Info => {
            show_all_info();
        }
        Commands::Temps => {
            show_temps();
        }
        Commands::Mode => {
            toggle_power_mode();
        }
        Commands::Fans { boost } => {
            if let Some(boost) = boost {
                set_both_fan_boosts(boost);
            } else {
                show_fan_boosts();
            }
        }
    };
}
