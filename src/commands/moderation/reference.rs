use poise::serenity_prelude::{ChannelId, MessageId, Timestamp, UserId};

use crate::{
    structs::{ActionTypes, CaseUpdateAction, CaseUpdateValue},
    utils::Valeriyya,
    Context, Error,
};

#[doc = "Reference two seperate cases."]
#[poise::command(
    slash_command,
    category = "Moderation",
    default_member_permissions = "MANAGE_GUILD",
    prefix_command,
    track_edits
)]
pub async fn reference(
    ctx: Context<'_>,
    #[description = "The case to assign a reference."] case: u32,
    #[description = "The reference case"] reference: u32,
) -> Result<(), Error> {
    let database = &ctx.data().database();
    let guild_id = ctx.guild_id().unwrap().get();
    let mut db = Valeriyya::get_database(database, guild_id).await?;

    let db_cases = db.cases.clone();

    let case_1 = db_cases.iter().find(|c| c.id == case);
    let case_2 = db_cases.iter().find(|c| c.id == reference);

    match (case_1, case_2) {
        (None, None) => {
            ctx.send(Valeriyya::reply("Cases with these IDs don't exist").ephemeral(true)).await?;
            return Ok(());
        }
        (Some(_), None) => {
            ctx.send(Valeriyya::reply(format!("Case with the ID: {} doesn't exist", reference)).ephemeral(true)).await?;
            return Ok(());
        }
        (None, Some(_)) => {
            ctx.send(Valeriyya::reply(format!("Case with the ID: {} doesn't exist", case)).ephemeral(true)).await?;
            return Ok(());
        }
        (Some(ref1), Some(ref2)) => {
            if ref1.id == ref2.id {
                ctx.send(Valeriyya::reply("You can't reference the same case").ephemeral(true)).await?;
                return Ok(());
            }
        }
    }

    let case_found = case_1.unwrap();

    db = db.update_case(
        case,
        CaseUpdateAction::Reference,
        CaseUpdateValue {
            reason: None,
            reference: Some(reference),
        },
    );

    if let Some(logs) = &db.channels.logs {
        let channel = ChannelId::new(logs.parse::<u64>().unwrap());
        if case_found.message.is_some() {
            let mut log_channel_msg = channel
                .message(
                    ctx.serenity_context(),
                    MessageId::new(
                        case_found
                            .message
                            .as_deref()
                            .unwrap()
                            .parse::<u64>()
                            .unwrap(),
                    ),
                )
                .await?;
            let staff_user_cache = UserId::new(case_found.staff_id.parse::<u64>().unwrap())
                .to_user(ctx.serenity_context())
                .await?
                .to_owned();
            let (staff_name, staff_id, staff_face) = (
                staff_user_cache.tag(),
                staff_user_cache.id,
                staff_user_cache.face(),
            );
            let target_user = UserId::new(case_found.target_id.parse::<u64>().unwrap())
                .to_user(ctx.serenity_context())
                .await?
                .tag();

            let icon_url = ctx
                .guild()
                .unwrap()
                .icon_url()
                .unwrap_or(String::from(""));

            let mut embed = Valeriyya::embed()
                .timestamp(Timestamp::from(&Timestamp::from_unix_timestamp(case_found.date).unwrap()))
                .author(Valeriyya::reply_author(format!("{} ({})", staff_name, staff_id)).icon_url(staff_face))
                .thumbnail(icon_url)
                .footer(Valeriyya::reply_footer(format!("Case {}", case_found.id)));
            
                let mut description = format!(
                    "Member: `{}`\nAction: `{:?}`\nReason: `{}`\nReference: `{}`",
                    target_user, case_found.action, case_found.reason, reference
                );
                
                if case_found.action == ActionTypes::Mute {
                    description += &format!("\nExpiration: {}", Valeriyya::time_format(case_found.expiration.unwrap().to_string()));
                }
                
                embed = embed.description(description);
            

            log_channel_msg.edit(ctx.serenity_context(), Valeriyya::msg_edit().embed(embed)).await?;
        };
    }

    ctx.send(Valeriyya::reply(format!("Updated case with the id: {case}")).ephemeral(true)).await?;
    db.execute(database).await;
    Ok(())
}
