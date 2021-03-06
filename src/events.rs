#[allow(unused_parens)]
use super::parse_time;
use super::storage;
use log::*;

use chrono::Utc;

extern crate job_scheduler;

use serenity::{
    model::{
        channel::{Message, Reaction, ReactionType},
        gateway::{Activity, Ready},
    },
    prelude::{Context, EventHandler},
};

pub struct Handler;
pub struct HandlerEmpty;

fn split_once(in_string: &str) -> (&str, &str) {
    let mut splitter = in_string.splitn(2, ':');
    let first = splitter.next().unwrap_or_default();
    let second = splitter.next().unwrap_or_default();
    (first, second)
}

impl EventHandler for HandlerEmpty {}

impl EventHandler for Handler {
    fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        let reaction_msg = reaction.message(&ctx.http).unwrap();
        match &reaction.emoji {
            ReactionType::Unicode(uni) => match uni.as_ref() {
                "👀" => {
                    use std::thread;

                    let message_content = &reaction_msg.content;
                    let msg_args: Vec<&str> = message_content.split_whitespace().collect();
                    debug!(
                        "Msg author {} reaction user {}",
                        reaction_msg.author.id, reaction.user_id
                    );

                    if msg_args.len() > 0 && msg_args[0] == "!remindme" {
                        let (_command, date_args) = msg_args.split_at(1);

                        let time_since_message = Utc::now()
                            .signed_duration_since(reaction_msg.timestamp)
                            .num_seconds();
                        let (reply_msg, time_to_wait_in_seconds, _used_args) =
                            parse_time::parse_for_wait_time(
                                time_since_message as i32,
                                Vec::from(date_args),
                            );
                        if reaction_msg.author.id == reaction.user_id
                            || reaction.user(&ctx).unwrap().bot
                        {
                            debug!("Bots and original user cannot be reminded with reaction.");
                        } else if time_to_wait_in_seconds <= 0 {
                            let dm_confirm =
                                reaction.user(&ctx).unwrap().direct_message(&ctx, |m| {
                                    m.content(format!("Reminder already passed."))
                                });

                            match dm_confirm {
                                Ok(_) => {}
                                Err(why) => {
                                    error!("Error sending completion notification DM {:?}", why);
                                }
                            }
                        } else {
                            let mut msg_url = String::from("Url not found");
                            if reaction_msg.is_private() {
                                msg_url = format!(
                                    "http://discordapp.com/channels/@me/{}/{}",
                                    reaction_msg.channel_id, reaction_msg.id
                                );
                            } else {
                                msg_url = format!(
                                    "http://discordapp.com/channels/{}/{}/{}",
                                    reaction_msg.guild_id.unwrap_or_default(),
                                    reaction_msg.channel_id,
                                    reaction_msg.id
                                );
                            }
                            // TODO Add rest of the arguments to the message
                            let remind_msg = format!("Reminder for link: {}", &msg_url);
                            let dm_confirm = reaction.user(&ctx).unwrap().direct_message(&ctx, |m| {
                                m.content(format!(
                                    "Reminder will be DMed in {} from original message date. Others can react with 👀 to also be reminded.",
                                    &reply_msg
                                ))
                            });
                            match dm_confirm {
                                Ok(_) => {}
                                Err(why) => {
                                    error!("Error sending confirmation notification DM {:?}", why);
                                }
                            }
                            let user_id = &reaction.user(&ctx).unwrap().id.0;
                            match storage::save_reminder(
                                reaction_msg.timestamp.timestamp(),
                                time_to_wait_in_seconds,
                                *user_id,
                                remind_msg.to_string(),
                            ) {
                                Ok(_x) => {}
                                Err(why) => {
                                    error!("Error saving reminder {:?}", why);
                                }
                            };
                            thread::sleep(std::time::Duration::new(
                                time_to_wait_in_seconds as u64,
                                0,
                            ));
                            let dm = &reaction
                                .user(&ctx)
                                .unwrap()
                                .direct_message(&ctx.http, |m| m.content(remind_msg));
                            match dm {
                                Ok(_) => {
                                    let _ = reaction_msg.react(&ctx, '✅');
                                }
                                Err(why) => {
                                    error!("Err sending DM: {:?}", why);
                                }
                            };
                        }
                    }
                    ()
                }
                _ => (),
            },
            _ => (),
        };
    }
    fn message(&self, ctx: Context, _new_message: Message) {
        if _new_message.content == "???" {
            use std::thread;
            thread::sleep(std::time::Duration::new(1, 0));

            let remind_msg = format!(
                "<@{}> wants to be reminded of something.",
                &_new_message.author.id
            );
            if let Err(err) = _new_message.channel_id.say(&ctx.http, remind_msg) {
                error!("Error giving message: {:?}", err);
            }
        }
    }
    fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is ready", ready.user.name);

        ctx.set_activity(Activity::playing(&String::from(
            "Oh dear! I shall be too late!",
        )));

        match storage::load_reminders(ctx) {
            Ok(_) => info!("Reminders loaded OK."),
            Err(why) => error!("Error loading reminders. {:?}", why),
        };
    }
}
