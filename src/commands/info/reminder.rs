use chrono::{Duration, Utc};
use humantime::parse_duration;

use crate::{structs::Reminder, utils::Valeriyya, Context, Error};

#[doc = "Reminders."]
#[poise::command(slash_command, prefix_command, category = "Info", subcommands("add", "remove", "list"), track_edits)]
pub async fn reminder(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, category = "Info", prefix_command, track_edits)]
pub async fn add(
    ctx: Context<'_>,
    message: String,
    time: String,
) -> Result<(), Error> {
    let database = &ctx.data().database();
    let guild_id = ctx.guild_id().unwrap().get();

    let mut db = Valeriyya::get_database(database, guild_id).await?;

    let human_time = parse_duration(&time).expect("There was an error calculating the human time!");
    let now = Utc::now();
    let future_time = now + Duration::from_std(human_time).unwrap();

    let reminder = Reminder {
        id: db.reminder_count + 1,
        user: ctx.author().id.get(),
        message: message,
        datetime: future_time,
        created_at: now,
        channel: ctx.channel_id().get(),
    };

    db.reminder_count += 1;
    db = db.add_reminder(reminder);

    db.execute(database).await;

    ctx.reply("Reminder created!").await?;
    Ok(())
}

#[poise::command(slash_command, category = "Info", prefix_command, track_edits)]
pub async fn remove(ctx: Context<'_>, id: u32) -> Result<(), Error> {
    let database = &ctx.data().database();
    let guild_id = ctx.guild_id().unwrap().get();

    let mut db = Valeriyya::get_database(database, guild_id).await?;
    let user_id = ctx.author().id.get();

    let reminder = db.get_reminder_by_id(id);
    if let Some(reminder) = reminder {
        if reminder.user != user_id {
            ctx.reply("You can only remove your own reminders!").await?;
            return Ok(());
        }

        db = db.remove_reminder(id);
        db.execute(database).await;

        ctx.reply("Reminder removed successfully!").await?;
    } else {
        ctx.reply("No reminder found with that ID!").await?;
    }

    Ok(())
}

#[poise::command(slash_command, category = "Info", prefix_command, track_edits)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let database = &ctx.data().database();
    let guild_id = ctx.guild_id().unwrap().get();

    let db = Valeriyya::get_database(database, guild_id).await?;
    let user_id = ctx.author().id.get();

    let reminders: Vec<Reminder> = db.get_reminders_for_user(user_id);

    if reminders.is_empty() {
        ctx.reply("You have no reminders!").await?;
        return Ok(());
    }

    let mut response = String::from("Here are your reminders:\n");
    for reminder in reminders {
        response.push_str(&format!(
            "**ID**: {}\n**Message**: {}\n**Time**: {}\n\n",
            reminder.id,
            reminder.message,
            reminder.datetime.format("%Y-%m-%d %H:%M:%S UTC")
        ));
    }

    ctx.reply(response).await?;
    Ok(())
}

