#![allow(unused)]

mod controller;

use std::{
    fs::{self, File, OpenOptions},
    io::{self, stdin, Read, Write},
    ops::Deref,
    os::unix::net::UnixListener,
    sync::{
        atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering},
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
            let signal = Arc::new(AtomicIsize::new(0));
            let sig_clone = signal.clone();
            print!("Update Interval: {interval} seconds and using fan curves from {path}\n");
            let t = thread::spawn(move || {
                let mut controller = Controller::new(&path);
                controller.watch(interval, &sig_clone);
            });

            let mut buf = String::with_capacity(1024);

            loop {
                let size = stdin().read_line(&mut buf).unwrap();
                if size != 0 {
                    let cmd = buf.trim();
                    if cmd == "q" {
                        print!("Exiting...\n");
                        signal.store(-1, Ordering::SeqCst);
                        break;
                    } else if cmd == "m" {
                        print!("Changing Power mode\n");
                        signal.store(1, Ordering::SeqCst);
                    } else if cmd == "i" || cmd == "s" {
                        print!("Showing Info...\n");
                        signal.store(2, Ordering::SeqCst);
                    } else if cmd == "r" {
                        print!("Reloading...\n");
                        signal.store(3, Ordering::SeqCst);
                    } else {
                        eprint!("Unknown command: {cmd}\n");
                    }

                    buf.clear();
                }
            }
            print!("Joining thread...\n");
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
