use chrono::{Duration, Utc};
use humantime::parse_duration;
use serenity::all::{
    ButtonStyle, ComponentInteractionCollector, CreateActionRow, CreateButton,
    CreateInteractionResponseMessage,
};

use crate::{structs::Reminder, utils::Valeriyya, Context, Error};

#[doc = "Reminders."]
#[poise::command(
    slash_command,
    prefix_command,
    category = "Info",
    subcommands("create", "list"),
    track_edits
)]
pub async fn reminder(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, category = "Info", prefix_command, track_edits)]
pub async fn create(ctx: Context<'_>, message: String, time: String) -> Result<(), Error> {
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
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let database = &ctx.data().database();
    let guild = ctx.guild_id().unwrap();
    let guild_id = guild.get();

    let mut db = Valeriyya::get_database(database, guild_id).await?;
    let user_id = ctx.author().id.get();

    let mut reminders: Vec<Reminder> = db.get_reminders_for_user(user_id);

    if reminders.is_empty() {
        ctx.reply("You have no reminders!").await?;
        return Ok(());
    }

    let mut component_buttons = vec![];
    let mut response = String::from("Here are your reminders:\n");
    for reminder in &reminders {
        response.push_str(&format!(
            "ID: {} - Message: {} - Time: {}\n",
            reminder.id,
            reminder.message,
            format!("<t:{}:R>", reminder.datetime.timestamp())
        ));
        component_buttons.push(
            CreateButton::new(format!("{}reminder{}", reminder.id, guild_id))
                .style(ButtonStyle::Danger)
                .label(format!("{}", reminder.id)),
        );
    }
    let component_row = CreateActionRow::Buttons(component_buttons.into());

    ctx.send(Valeriyya::reply(response).components(vec![component_row]))
        .await?;

    while let Some(mci) = ComponentInteractionCollector::new(ctx.serenity_context())
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(120))
        .await
    {
        let custom_id_str = mci.data.custom_id.clone();
        let id_str = custom_id_str.split("reminder").next().unwrap_or("");
        if let Ok(custom_id) = id_str.parse::<u32>() {
            reminders.retain(|r| r.id == custom_id);

            db = db.remove_reminder(custom_id);

            mci.create_response(
                ctx.http(),
                serenity::all::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("Reminder {} removed successfully!", custom_id)),
                ),
            )
            .await?;
            break;
        } else {
            mci.create_response(
                ctx.http(),
                serenity::all::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("Failed to parse reminder ID."),
                ),
            )
            .await?;
            break;
        }
    }

    db.execute(database).await;

    Ok(())
}
