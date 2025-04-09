use crate::{
    structs::{ActionTypes, Case},
    utils::{member_managable, Valeriyya},
    Context, Error,
};
use poise::serenity_prelude::{ChannelId, Timestamp, UserId, Member};

#[doc = "Bans a member from the guild."]
#[poise::command(
    slash_command,
    category = "Moderation",
    default_member_permissions = "BAN_MEMBERS",
    prefix_command,
    track_edits
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "The member to ban"]
    #[rename = "member"]
    mem: Option<Member>,
    #[description = "The member to ban (Use this to provide an id instead of mention)"]
    member_id: Option<String>,
    #[description = "The reason for this ban."]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let database = &ctx.data().database();
    let guild = ctx.guild_id().unwrap();
    let guild_id = guild.get();
    let icon_url = ctx
        .guild()
        .unwrap()
        .icon_url()
        .unwrap_or(String::from(""));

    let mut guild_db = Valeriyya::get_database(database, guild_id).await?;
    let case_number = guild_db.cases_number + 1;
    let reason_default = reason.unwrap_or(format!("Use /reason {} <...reason> to set a reason for this case.", case_number));

    if let Some(member) = &mem {
        if !member_managable(ctx, member).await {
            ctx.send(Valeriyya::reply("The member can't be managed so you can't ban them!").ephemeral(true)).await?;
            return Ok(());
        }
        if guild
            .bans(ctx.http(), None, None)
            .await?
            .iter()
            .any(|ban| ban.user.id == member.user.id)
        {

            ctx.send(Valeriyya::reply("This member is already banned from this guild.").ephemeral(true)).await?;
        }
        member
            .ban(ctx.http(), 7, Some(&reason_default))
            .await?;

        let message = if guild_db.channels.logs.as_ref().is_some() {
            let sent_msg = ChannelId::new(
                guild_db
                    .channels
                    .logs
                    .as_ref()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap(),
            )
            .send_message(ctx.http(), Valeriyya::msg_reply().add_embed(
                Valeriyya::embed()
                    .author(Valeriyya::reply_author(format!(
                        "{} ({})",
                        ctx.author().tag(),
                        ctx.author().id
                    )).icon_url(ctx.author().face()))
                    .thumbnail(&icon_url)
                    .description(format!(
                        "Member: `{}`\nAction: `{:?}`\nReason: `{}`",
                        member.user.tag(),
                        ActionTypes::Ban,
                        reason_default
                    ))
                    .footer(Valeriyya::reply_footer(format!("Case {}", case_number)))
            )).await.expect("Guild log channel doesn't exist");

            Some(sent_msg.id.to_string())
        } else {
            None
        };

        guild_db = guild_db.add_cases(Case {
            id: case_number,
            action: ActionTypes::Ban,
            guild_id: guild_id.to_string(),
            staff_id: ctx.author().id.to_string(),
            target_id: member.user.id.to_string(),
            date: Timestamp::unix_timestamp(&Timestamp::now()),
            reason: reason_default.to_string(),
            message,
            expiration: None,
            reference: None,
        });

        ctx.say(format!(
            "{:?} has been banned by {:?}!",
            member,
            ctx.author()
        ))
        .await?;
    } else if let Some(m_id) = &member_id {
        let user_id = UserId::new(m_id.parse().unwrap());
        if guild
            .bans(ctx.http(), None, None)
            .await?
            .iter()
            .any(|ban| ban.user.id == user_id)
        {
            ctx.send(Valeriyya::reply("This member is already banned from this guild.").ephemeral(true)).await?;
        }
        guild
            .ban(ctx.http(), user_id, 7, Some(&reason_default))
            .await?;

        let message = if guild_db.channels.logs.as_ref().is_some() {
            let sent_msg = ChannelId::new(
                guild_db
                    .channels
                    .logs
                    .as_ref()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap(),
            )
            .send_message(ctx.http(), Valeriyya::msg_reply().add_embed(
                Valeriyya::embed()
                    .author(Valeriyya::reply_author(format!(
                        "{} ({})",
                        ctx.author().tag(),
                        ctx.author().id
                    )).icon_url(ctx.author().face()))
                    .thumbnail(&icon_url)
                    .description(format!(
                        "Member: `{}`\nAction: `{:?}`\nReason: `{}`",
                        m_id,
                        ActionTypes::Ban,
                        reason_default
                    ))
                    .footer(Valeriyya::reply_footer(format!("Case {}", case_number)))
            )).await.expect("Guild log channel doesn't exist");

            Some(sent_msg.id.to_string())
        } else {
            None
        };

        guild_db = guild_db.add_cases(Case {
            id: case_number,
            action: ActionTypes::Ban,
            guild_id: guild_id.to_string(),
            staff_id: ctx.author().id.to_string(),
            target_id: user_id.to_string(),
            date: Timestamp::unix_timestamp(&Timestamp::now()),
            reason: reason_default.to_string(),
            message,
            expiration: None,
            reference: None,
        });

        ctx.say(format!(
            "Member with the the id: {} has been banned by {:?}!",
            user_id,
            ctx.author()
        ))
        .await?;
    }
    guild_db.execute(database).await;
    Ok(())
}
