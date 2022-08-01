use std::num::NonZeroU64;

use crate::{
    serenity,
    utils::{self, create_case, get_guild_db, member_managable, ActionTypes, Case},
    Context, Error,
};
use poise::{
    serenity_prelude::{
        ChannelId, CreateMessage, Timestamp, UserId,
    },
    CreateReply,
};

/// Bans a member from the guild.
#[poise::command(
    slash_command,
    category = "Moderation",
    default_member_permissions = "BAN_MEMBERS"
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "The member to ban"] member: Option<serenity::Member>,
    #[description = "The member to ban (Use this to provide an id instead of mention)"]
    member_id: Option<String>,
    #[description = "The reason for this ban."]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let database = &ctx.data().database;
    let guild_id = ctx.guild_id().unwrap();
    let icon_url = ctx
    .guild()
    .unwrap()
    .icon_url()
    .unwrap_or_else(|| String::from(""));
    
    let db = get_guild_db(database, guild_id).await;
    let reason_default = reason.unwrap_or_else(|| format!("Use /reason {} <...reason> to seat a reason for this case.", db.cases_number + 1));

    if let Some(m) = &member {
        if !member_managable(ctx, m).await {
            ctx.send(
                CreateReply::default()
                    .content("The member can't be managed so you can't ban them!")
                    .ephemeral(true),
            )
            .await;
            return Ok(());
        }
        if guild_id
            .bans(&ctx.discord().http)
            .await?
            .iter()
            .any(|ban| ban.user.id == m.user.id)
        {
            ctx.send(
                CreateReply::default()
                    .content("This member is already banned from this guild.")
                    .ephemeral(true),
            )
            .await;
        }
        m.ban_with_reason(ctx.discord(), 7, &reason_default).await?;

        let message = if db.channels.logs.as_ref().is_some() {
            let sent_msg = ChannelId(db.channels.logs.as_ref().unwrap().parse::<NonZeroU64>().unwrap()).send_message(
                ctx.discord(),
                CreateMessage::default().add_embed(
                    utils::valeriyya_embed()
                    .author(
                        serenity::CreateEmbedAuthor::default()
                            .name(format!("{} ({})", ctx.author().tag(), ctx.author().id))
                            .icon_url(ctx.author().face()),
                    )
                        .thumbnail(&icon_url)
                        .description(format!(
                            "Member: `{}`\nAction: `{:?}`\nReason: `{}`",
                            m.user.tag(),
                            ActionTypes::ban,
                            reason_default
                        ))
                        .footer(serenity::CreateEmbedFooter::default().text(format!("Case {}", db.cases_number + 1)))
                ),
            )
            .await?;
            Some(sent_msg.id.to_string())
        } else {
            None
        };

        create_case(
            database,
            guild_id,
            Case {
                id: db.cases_number + 1,
                action: ActionTypes::ban,
                guild_id: guild_id.to_string(),
                staff_id: ctx.author().id.to_string(),
                target_id: m.user.id.to_string(),
                date: Timestamp::unix_timestamp(&Timestamp::now()),
                reason: reason_default.to_string(),
                message,
                expiration: None,
                reference: None,
            },
        )
        .await;

        ctx.say(format!("{:?} has been banned by {:?}!", member, ctx.author())).await;
    }
    if let Some(m_id) = &member_id {
        let user_id = UserId(m_id.parse().unwrap());
        if guild_id
            .bans(&ctx.discord().http)
            .await?
            .iter()
            .any(|ban| ban.user.id == user_id)
        {
            ctx.send(
                CreateReply::default()
                    .content("This member is already banned from this guild.")
                    .ephemeral(true),
            )
            .await;
        }
        guild_id
            .ban_with_reason(ctx.discord(), user_id, 7, &reason_default)
            .await?;

            let message = if db.channels.logs.is_some() {
                let sent_msg = ChannelId(db.channels.logs.unwrap().parse::<NonZeroU64>().unwrap()).send_message(
                    ctx.discord(),
                    CreateMessage::default().add_embed(
                        utils::valeriyya_embed()
                        .author(
                            serenity::CreateEmbedAuthor::default()
                                .name(format!("{} ({})", ctx.author().tag(), ctx.author().id))
                                .icon_url(ctx.author().face()),
                        )
                            .thumbnail(&icon_url)
                            .description(format!(
                                "Member: `{}`\nAction: `{:?}`\nReason: `{}`",
                                m_id,
                                ActionTypes::ban,
                                reason_default
                            ))
                            .footer(serenity::CreateEmbedFooter::default().text(format!("Case {}", db.cases_number + 1)))
                    ),
                )
                .await?;
                Some(sent_msg.id.to_string())
            } else {
                None
            };

        create_case(
            database,
            guild_id,
            Case {
                id: db.cases_number + 1,
                action: ActionTypes::ban,
                guild_id: guild_id.to_string(),
                staff_id: ctx.author().id.to_string(),
                target_id: user_id.0.to_string(),
                date: Timestamp::unix_timestamp(&Timestamp::now()),
                reason: reason_default.to_string(),
                message,
                expiration: None,
                reference: None,
            },
        )
        .await;
        ctx.say(format!("Member with the the id: {} has been banned by {:?}!", user_id, ctx.author())).await;
    }

    Ok(())
}
