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
            let p = path.clone();
            let mut buf = String::with_capacity(1024);
            {
                OpenOptions::new()
                    .read(true)
                    .open(path)
                    .unwrap()
                    .read_to_string(&mut buf)
                    .unwrap();
            }

            let (cpu_graph, gpu_graph) = get_coords_from_string(&buf);

            let alien_dev_infos = get_alien_dev_graph_info(cpu_graph.clone(), gpu_graph.clone());
            print!("Update Interval: {interval} seconds and using fan curves from {p}\n");

            let mut t = Some(thread::spawn(move || {
                let mut controller = Controller::new(alien_dev_infos);
                controller.watch(interval, &sig_clone);
            }));

            loop {
                buf.clear();
                let size = stdin().read_line(&mut buf).unwrap();
                if size != 0 {
                    let cmd = buf.trim();
                    match cmd {
                        "q" => {
                            print!("Exiting...\n");
                            signal.store(-1, Ordering::SeqCst);
                            break;
                        }
                        "m" => {
                            print!("Changing Power mode\n");
                            signal.store(1, Ordering::SeqCst);
                        }
                        "i" | "s" => {
                            print!("Showing Info...\n");
                            signal.store(2, Ordering::SeqCst);
                        }
                        "r" => {
                            print!("Reloading...\n");
                            signal.store(3, Ordering::SeqCst);
                        }
                        "n" => {
                            signal.store(4, Ordering::SeqCst);
                        }
                        "p" => {
                            if let Some(t) = t.take() {
                                signal.store(-1, Ordering::SeqCst);
                                t.join();
                                print!("Paused Watch\n");
                            } else {
                                signal.store(0, Ordering::SeqCst);
                                let sig_clone = signal.clone();
                                let alien_dev_infos =
                                    get_alien_dev_graph_info(cpu_graph.clone(), gpu_graph.clone());
                                t = Some(thread::spawn(move || {
                                    let mut controller = Controller::new(alien_dev_infos);
                                    controller.watch(interval, &sig_clone);
                                }));
                                print!("Resumed Watch\n");
                            }
                        }
                        _ => {
                            eprint!("Unknown command: {cmd}\n");
                        }
                    }
                }
            }
            print!("Joining thread...\n");
            if let Some(t) = t {
                t.join();
            }
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
