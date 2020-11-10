mod announce;
mod events;
mod parse_time;
mod storage;

use events::Handler;

use serenity::{
    client::Client,
    framework::standard::Args,
    framework::standard::{
        macros::{command, group},
        CommandResult, StandardFramework,
    },
    model::channel::Message,
    prelude::{Context},
};
use std::fs::File;
use std::io::prelude::*;

#[group]
#[commands(help, ping, remindme)]
struct General;

fn main() {
    let mut file = File::open(".token").unwrap();
    let mut token = String::new();
    file.read_to_string(&mut token)
        .expect("Token could not be read");

    let mut client = Client::new(&token, Handler).expect("Error creating client");

    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("!"))
            .group(&GENERAL_GROUP),
    );
    if let Err(msg) = client.start() {
        println!("Error: {:?}", msg);
    }
}

#[command]
fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    match msg.reply(ctx, "Pong") {
        _ => {}
    };

    Ok(())
}

#[command]
fn help(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    match msg
        .channel_id
        .say(&ctx.http, "Available commands: \n * remindme ")
    {
        _ => {}
    };
    Ok(())
}

#[command]
fn remindme(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    use chrono::Utc;
    use std::thread;

    let args_list = args.raw().collect::<Vec<&str>>();

    let time_since_message = Utc::now()
        .signed_duration_since(msg.timestamp)
        .num_seconds();

    let (reply_msg, time_to_wait_in_seconds, used_args) =
        parse_time::parse_for_wait_time(time_since_message as i32, args_list);

    for _ in 0..used_args {
        // Consume the arguments that were processed above
        args.advance();
    }

    if time_to_wait_in_seconds > 0 {
        let msg_private = msg.is_private();

        let message_stamp = msg.timestamp.timestamp();
        let user_id = msg.author.id.0;

        let dm_confirm = msg.author.direct_message(&ctx, |m| {
            m.content(format!("Reminder will be DMed in {}.{}", &reply_msg, {
                if (!msg_private) {
                    " Others can react with 👀 to also be reminded."
                } else {
                    ""
                }
            }))
        });

        let _ = msg.react(&ctx, '👀');
        let mut msg_url = String::from("Url not found");
        if msg_private {
            msg_url = format!(
                "http://discordapp.com/channels/@me/{}/{}",
                msg.channel_id, msg.id
            );
        } else {
            msg_url = format!(
                "http://discordapp.com/channels/{}/{}/{}",
                msg.guild_id.unwrap_or_default(),
                msg.channel_id,
                msg.id
            );
        }
        let remind_msg = format!("Reminder: \"{}\" \nLink: {}", args.rest(), &msg_url);
        match storage::save_reminder(
            message_stamp,
            time_to_wait_in_seconds,
            user_id,
            remind_msg.to_string(),
        ) {
            Ok(_x) => {}
            Err(why) => println!("Error saving remider. {:?}", why),
        };

        //// Alternative way to mention player instead of `msg.reply`
        // let remind_msg = format!(
        //     "Reminder <@{}>: {} \nLink: {}",
        //     &msg.author.id,
        //     args.rest(),
        //     &msg_url
        // );

        thread::sleep(std::time::Duration::new(time_to_wait_in_seconds as u64, 0));

        let dm_reminder = msg.author.direct_message(&ctx, |m| m.content(remind_msg));
        // let dm_reminder = ctx
        //     .http
        //     .get_user(user_id)?
        //     .direct_message(&ctx, |m| m.content(remind_msg));

        match dm_reminder {
            Ok(_) => {
                let _ = msg.react(&ctx, '✅');
                // let _ = msg.react(&ctx, '👌');
            }
            Err(why) => {
                println!("Err sending DM: {:?}", why);

                // let _ = msg.reply(&ctx, "There was an error DMing you help.");
            }
        };
    } else {
        match msg.channel_id.say(&ctx.http, format!("{}", &reply_msg)) {
            Ok(_x) => {}
            Err(why) => {
                println!("Error when telling user about parse error. {:?}", why);
            }
        };
    }

    Ok(())
}
