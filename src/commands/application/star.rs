use crate::{utils::Valeriyya, Context, Error};

use poise::serenity_prelude as serenity;
use ::serenity::all::CreateAttachment;

#[doc = "Star the message and send it to the starboard channel!"]
#[poise::command(context_menu_command = "Star", category = "Application|Message")]
pub async fn star(
    ctx: Context<'_>,
    #[description = "Message to star "] msg: serenity::Message,
) -> Result<(), Error> {
    let database = &ctx.data().database();
    let guild_id = ctx.guild_id().unwrap().get();
    let guild_db = Valeriyya::get_database(database, guild_id).await?;

    let channel_id = ctx.channel_id();
    if guild_db.channels.starboard.is_some() {
        let webhook = channel_id
            .create_webhook(
                ctx.http(),
                serenity::CreateWebhook::new(msg.author.display_name()).avatar(&CreateAttachment::url(ctx.http(), msg.author.avatar_url().unwrap(), "profile").await.unwrap()),
            )
            .await
            .unwrap();

        

        let mut files = Vec::new();
        
        for attachment in msg.attachments.iter() {
            let url = attachment.url.clone();
            let filename = attachment.filename.clone();
            
            match serenity::CreateAttachment::url(ctx.http(), url.as_str(), filename)
            .await
            {
                Ok(file) => files.push(file),
                Err(err) => {
                    tracing::error!("Failed to create attachment: {:?}", err);
                }
            }
        }
        
        ctx.reply("Successfully starred the message!").await?;
        webhook
            .execute(
                ctx.http(),
                false,
                serenity::ExecuteWebhook::new()
                    .content(format!("{}\n\nOriginal Message: {}", msg.content, msg.link()))
                    .add_files(files),
            )
            .await
            .unwrap();

        webhook.delete(ctx.http(), Some("Starred a Message")).await?;

    } else {
        ctx.reply("There is no starboard channel set!\nUse `settings channels starboard <channel>`").await?;
    };

    Ok(())
}
