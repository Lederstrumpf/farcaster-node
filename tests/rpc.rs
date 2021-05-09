#![cfg(feature = "integration_test")]

#[macro_use]
extern crate log;

use clap::Clap;
use farcaster_node::rpc::Client;
use farcaster_node::{Config, LogStyle};

use farcaster_node::cli::Command;
use microservices::shell::Exec;

#[test]
fn spawn_swap() {
    use farcaster_node::{cli::Opts, farcasterd::launch};
    let data_dir = vec!["-d", "tests/.farcaster_node"];
    let mut opts = Opts::parse_from(
        vec!["swap-cli"]
            .into_iter()
            .chain(data_dir.clone())
            .chain(vec!["make"]),
    );
    opts.process();
    info!("opts: {:?}", opts);
    let config: Config = opts.shared.clone().into();

    let mut client = Client::with(config.clone(), config.chain.clone())
        .expect("Error running client");

    let mut farcasterd = launch("../farcasterd", data_dir.clone()).unwrap();

    use std::{thread, time};
    thread::sleep(time::Duration::from_secs_f32(0.5));

    Command::Ls
        .exec(&mut client)
        .unwrap_or_else(|err| eprintln!("{} {}", "error:".err(), err.err()));

    info!("executing command: {:?}", opts.command);
    opts.command
        .exec(&mut client)
        .unwrap_or_else(|err| eprintln!("{} {}", "error:".err(), err.err()));

    opts = Opts::parse_from(
        vec!["swap-cli"]
            .into_iter()
            .chain(data_dir.clone())
            .chain(vec![
                "make", "Testnet", "Bitcoin", "Monero", "101", "100", "10",
                "30", "20", "Alice", "0.0.0.0", "9376",
            ]),
    );
    opts.command
        .exec(&mut client)
        .unwrap_or_else(|err| eprintln!("{} {}", "error:".err(), err.err()));

    Command::Pedicide
        .exec(&mut client)
        .unwrap_or_else(|err| eprintln!("{} {}", "error:".err(), err.err()));

    farcasterd.kill().expect("Couldn't kill farcasterd");
    let ps_out = std::process::Command::new("ps")
        .args(&["-e"])
        .output()
        .expect("failed to execute process")
        .stdout;

    let ps_out_grep = std::process::Command::new("grep")
        .args(&[
            "-e",
            "'farcasterd\\|peerd'",
            std::str::from_utf8(&ps_out).unwrap(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e))
        .stdout;

    // fails if there are any lingering `farcasterd`s or `peerd`s, so this test is a false-negative if any are running independent of the test
    assert_eq!(ps_out_grep.len(), 0);

}
