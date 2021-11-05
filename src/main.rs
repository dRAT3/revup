extern crate clap;
use clap::{App, Arg, ArgGroup};
use dotenv;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Deserialize)]
struct Commando {
    command: String,
    args: Vec<String>,
    envs: Vec<String>,
}
#[derive(Serialize, Deserialize)]
struct Commandos {
    commands: Vec<Commando>,
}

fn main() {
    let matches =
        App::new("revup")
            .version("v0.0.1")
            .author("author: dRAT3")
            .about(
                "
Sets up the rev2 simulator for calling functions instantly, looks for .revup file
in the current dir, and runs the rev2 commands in order storing the created entities
address locations in a dotenv file. Run ./envup.sh to access the .env variables
from the parent shell.

Currently windows isn't supported. Pull requests for windows are welcome!
",
            )
            .arg(
                Arg::with_name("file")
                    .short("f")
                    .takes_value(true)
                    .help("Uses a custom .revup file"),
            )
            .arg(Arg::with_name("init").short("i").help(
                "Creates a default config file in the working directory, and the envup.sh file",
            ))
            .arg(Arg::with_name("reset").short("r").help(
                "Resets the simulator, creates a new account and stores the value in $account",
            ))
            .group(
                ArgGroup::with_name("group")
                    .args(&["file", "reset", "init"])
                    .required(false),
            )
            .get_matches();

    if matches.is_present("file") {
        let path = Path::new(matches.value_of("file").unwrap());
        run_file(path.to_path_buf());
    } else if matches.is_present("reset") {
        run_reset();
    } else if matches.is_present("init") {
        run_init();
    } else {
        match run().err() {
            Some(e) => println!("{}", e),
            None => {}
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut cur_dir = std::env::current_dir()?;
    cur_dir.push(".revup");

    if !cur_dir.exists() {
        println!(".revup file not found, run --init to create a default .revup file");
        std::process::exit(0);
    }

    match run_file(cur_dir).err() {
        Some(e) => println!("Error while executing commands \n{}", e),
        None => {}
    }
    Ok(())
}

fn run_file(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    //Clear env vars
    {
        let _dot_env = std::fs::File::create(".env")?;
    }

    let file = std::fs::File::open(path)?;
    let json: Commandos = serde_json::from_reader(file)?;

    for cmd in json.commands {
        run_cmd(cmd.command, cmd.args, cmd.envs)?;
    }

    Ok(())
}

fn run_reset() {
    /*
    //Reset ledger state
    println!(">>>rev2 reset");
    let reset = Command::new("rev2")
        .arg("reset")
        .output()
        .expect("failed to execute rev2");
    println!("{}", String::from_utf8_lossy(&reset.stdout).to_string());
    println!("{}", String::from_utf8_lossy(&reset.stderr).to_string());

    assert!(reset.status.success());

    println!(">>>rev2 new-account");
    //Create account and export value
    let create = Command::new("rev2")
        .arg("new-account")
        .output()
        .expect("failed to execute rev2");
    println!("{}", String::from_utf8_lossy(&create.stdout).to_string());
    println!("{}", String::from_utf8_lossy(&create.stderr).to_string());
    assert!(create.status.success());

    //Might not work on windows
    let res = walk_entities(String::from_utf8_lossy(&create.stdout).to_string());

    let account;
    match res {
        Ok(v) => {
            account = v[0].to_string();
        }
        Err(e) => {
            println!("Couldn't find account, exiting");
            std::process::exit(1);
        }
    }

    let mut arg = "account=".to_string();
    println!(">>> export {}", arg);
    */
}

fn run_init() {
    match std::env::current_dir() {
        Ok(mut dir) => {
            dir.push(".revup");
            if !dir.exists() {
                match create_default_config_file() {
                    Ok(_v) => println!("Default config file created in working directory"),
                    Err(e) => println!("Error while creating config file \n{}", e),
                }
            } else {
                println!(".revup file already exists remove it first, skipping");
            }
        }
        Err(e) => println!("Error: couldn't access working directory \n{}", e),
    }
}

fn run_cmd(
    command: String,
    args: Vec<String>,
    envs: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let res;
    if !args.is_empty() {
        res = Command::new("rev2").arg(&command).args(&args).output()?;
    } else {
        res = Command::new("rev2").arg(&command).output()?;
    }
    println!("{}", String::from_utf8_lossy(&res.stdout).to_string());
    println!("{}", String::from_utf8_lossy(&res.stderr).to_string());

    if !envs.is_empty() {
        let entities = walk_entities(String::from_utf8_lossy(&res.stdout).to_string())?;

        for (ent_it, env_it) in entities.iter().zip(envs.iter()) {
            println!("{}={}", env_it, ent_it);
            let _res = append_env(env_it.to_string(), ent_it.to_string())?;
        }
    }
    Ok(())
}

fn walk_entities(stdout: String) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    //Quick and dirty
    let mut ret_vec: Vec<String> = Vec::new();
    let location: usize;

    match stdout.rfind("New Entities") {
        Some(loc) => location = loc,
        None => return Err("No entities found".into()),
    }

    let substr_entities = &stdout[location..];
    let lines: Vec<&str> = substr_entities.lines().collect();

    for line in lines {
        if line.starts_with("└─ Component: ")
            || line.starts_with("└─ ResourceDef: ")
            || line.starts_with("└─ Package: ")
        {
            let entity_vec: Vec<&str> = line.split_whitespace().collect();
            let entity = entity_vec[2].to_string();
            ret_vec.push(entity);
        }
    }

    if ret_vec.len() < 1 {
        return Err("No entities found".into());
    }

    Ok(ret_vec)
}

fn append_env(mut env: String, ent: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut dotenv = std::fs::OpenOptions::new().append(true).open(".env")?;
    env.push_str("=");
    env.push_str(&ent);
    env.push_str("\n");
    Ok(dotenv.write_all(env.as_bytes())?)
}

fn create_default_config_file() -> Result<(), Box<dyn std::error::Error>> {
    let mut vector: Vec<Commando> = Vec::new();
    let reset = Commando {
        command: "reset".to_owned(),
        args: [].to_vec(),
        envs: [].to_vec(),
    };
    vector.push(reset);

    let account = Commando {
        command: "new-account".to_owned(),
        args: [].to_vec(),
        envs: ["account".to_string()].to_vec(),
    };
    vector.push(account);

    let token1 = Commando {
        command: "new-resource-fixed".to_owned(),
        args: [
            "10000".to_string(),
            "--name".to_string(),
            "emunie".to_string(),
            "--symbol".to_string(),
            "EMT".to_string(),
        ]
        .to_vec(),
        envs: ["token1".to_string()].to_vec(),
    };
    vector.push(token1);

    let token2 = Commando {
        command: "new-resource-fixed".to_owned(),
        args: [
            "10000".to_string(),
            "--name".to_string(),
            "gmunie".to_string(),
            "--symbol".to_string(),
            "GMT".to_string(),
        ]
        .to_vec(),
        envs: ["token2".to_string()].to_vec(),
    };
    vector.push(token2);
    /*
    let publish = Commando {
        command: "publish".to_owned(),
        args: [".".to_string()].to_vec(),
        envs: ["package".to_string()].to_vec(),
    };
    vector.push(publish);
    */
    let commandos = Commandos { commands: vector };

    let revup = std::fs::File::create(".revup")?;
    let ret = serde_json::to_writer_pretty(revup, &commandos)?;
    Ok(ret)
}
