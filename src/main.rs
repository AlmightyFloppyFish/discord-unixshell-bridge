extern crate discord;

mod buffer;
use buffer::ShellBuffer;

use discord::model::{ChannelId, Event, Message, MessageId, UserId};
use discord::Discord;

use std::env::var;
use std::io::{BufRead, BufReader};
use std::process::Command as bash;
use std::process::Stdio;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    // Log in to Discord using a bot token from the environment
    let discord = Discord::from_bot_token(&var("DISCORD_TOKEN").expect("No discord token"))
        .expect("login failed");

    // Establish and use a websocket connection
    let (mut connection, _) = discord.connect().expect("connect failed");
    println!("Successfully connected");

    let shell_session = Arc::new(Mutex::new(ShellBuffer::new()));

    loop {
        match connection.recv_event() {
            Ok(Event::MessageCreate(m)) => {
                let command = {
                    if m.content.chars().next().or(Some('.')).unwrap() == '$' {
                        if m.content.len() < 4 {
                            continue;
                        };
                        if m.author.id != UserId(220120722939445248)
                            && m.author.id != UserId(181200309949825024)
                        {
                            discord.send_message(
                                m.channel_id,
                                "Ni andra får använda detta när jag sattit upp det i en VM",
                                "",
                                false,
                            );
                            continue;
                        }
                        let bytes: Vec<char> = m.content.trim().chars().collect();
                        let tty: u8 = match bytes[1].to_string().parse() {
                            Ok(n) => n,
                            Err(_) => {
                                discord
                                    .send_message(
                                        m.channel_id,
                                        "Parse error: no tty specified",
                                        "",
                                        false,
                                    )
                                    .unwrap();
                                continue;
                            }
                        };
                        if !(bytes[2] == '(' && *bytes.last().unwrap() == ')') {
                            Command::Invalid(
                                "Parse error: invalid parenthesis, syntax is $`tty_number`(`shell_format`)".to_owned(),
                            )
                        } else {
                            Command::Shell(
                                bytes[3..bytes.len() - 1]
                                    .iter()
                                    .cloned()
                                    .collect::<String>(),
                                tty,
                            )
                        }
                    } else {
                        Command::Invalid("".to_owned())
                    }
                };
                println!("a");

                act(&discord, m, command, shell_session.clone());
            }
            Ok(_) => {}
            Err(discord::Error::Closed(code, body)) => {
                println!("Gateway closed on us with code {:?}: {}", code, body);
                break;
            }
            Err(err) => println!("Receive error: {:?}", err),
        };
    }
}

enum Command {
    Shell(String, u8),
    Invalid(String),
}

fn act(dg: &Discord, m: Message, command: Command, shell_session: Arc<Mutex<ShellBuffer>>) {
    match command {
        Command::Shell(command, tty_num) => {
            let com = bash::new("sh")
                .arg("-c")
                .arg(command.clone())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap()
                .stdout
                .ok_or_else(|| {
                    "Could not capture standard output";
                    return;
                })
                .unwrap();

            thread::spawn(move || {
                let reader = BufReader::new(com);
                reader.lines().for_each(|line| {
                    let mut sh = shell_session.lock().unwrap();
                    sh.write_to(tty_num, &line.unwrap(), m.channel_id);
                    sh.display(tty_num);
                });
                let mut sh = shell_session.lock().unwrap();
                sh.write_to(
                    tty_num,
                    &format!(" -:- Command \"{}\" exited -:- ", command),
                    m.channel_id,
                );
                sh.display(tty_num);
                drop(sh);
            });
        }
        Command::Invalid(text) => {
            if !text.is_empty() {
                dg.send_message(m.channel_id, &text, "", false).unwrap();
            }
        }
    }
}
