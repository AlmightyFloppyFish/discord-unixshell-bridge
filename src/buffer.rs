use std::collections::HashMap;
use std::default::Default;
use std::io::{BufRead, BufReader};
use std::iter;

use std::time::{Duration, SystemTime};

use discord::{
    model::{ChannelId, MessageId},
    Discord,
};
use std::sync::{mpsc::Sender, Arc, Mutex};

pub struct ShellBuffer {
    discord: Discord,
    pub ttys: HashMap<u8, Option<TTY>>,
}

impl ShellBuffer {
    pub fn new() -> Self {
        let mut ttys = HashMap::new();
        for i in 0..10 as u8 {
            ttys.insert(i, None);
        }

        let dg =
            Discord::from_bot_token(&std::env::var("DISCORD_TOKEN").expect("No discord token"))
                .unwrap();

        ShellBuffer {
            discord: dg,
            ttys: ttys,
        }
    }

    fn get_tty(&mut self, n: u8) -> &mut Option<TTY> {
        self.ttys.get_mut(&n).unwrap()
    }

    pub fn write_to(&mut self, tty_num: u8, line: &str, channel_id: ChannelId) {
        match self.get_tty(tty_num) {
            Some(tty) => {
                tty.write(line);
            }
            None => {
                // Doesn't exist, lets create it
                let m = self
                    .discord
                    .send_message(channel_id, &format!("**New TTY:** {}", tty_num), "", false)
                    .expect(
                        "Could not send message response, breaking here to prevent recursion leak",
                    );
                let new = TTY::new(m.channel_id, m.id);
                self.set_tty(tty_num, Some(new));
                self.write_to(tty_num, line, channel_id);
            }
        };
    }

    pub fn set_tty(&mut self, n: u8, tty: Option<TTY>) {
        let old = self.ttys.get_mut(&n).unwrap();
        *old = tty;
    }

    pub fn display(&mut self, tty_num: u8) {
        let mut tty = match self.get_tty(tty_num) {
            Some(tty) => tty,
            None => {
                eprintln!("TTY not open: {}", tty_num);
                return;
            }
        };
        if tty.last_display.elapsed().unwrap() < Duration::from_secs(2) {
            return;
        }
        let (cid, mid) = (tty.channel_id, tty.message_id);
        let mut content = tty.text.clone().to_vec();

        content[0] = format!("**TTY {}**\n```\n", tty_num);
        content.push("\n```".to_owned());

        tty.last_display = SystemTime::now();
        self.discord
            .edit_message(cid, mid, &content.join("\n"))
            .unwrap();
    }
}

pub struct TTY {
    last_display: SystemTime,
    channel_id: ChannelId,
    message_id: MessageId,
    text: [String; 20],
}

impl TTY {
    fn new(channel_id: ChannelId, message_id: MessageId) -> Self {
        let mut empties: [String; 20] = std::default::Default::default();
        for i in 0..empties.len() {
            empties[i] = "-> ".to_owned();
        }
        TTY {
            last_display: SystemTime::UNIX_EPOCH,
            channel_id: channel_id,
            message_id: message_id,
            text: empties,
        }
    }

    fn write(&mut self, line: &str) {
        for i in 0..self.text.len() {
            if i == 0 {
                continue;
            }
            self.text[i - 1] = self.text[i].clone();
        }
        (self.text[19] = format!("-> {}", line));
    }
}
