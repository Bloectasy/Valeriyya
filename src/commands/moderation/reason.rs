use poise::serenity_prelude::{ChannelId, MessageId, Timestamp, UserId};

use crate::{
    structs::{ActionTypes, CaseUpdateAction, CaseUpdateValue},
    utils::Valeriyya,
    Context, Error,
};

#[doc = "Changes the reason of a case."]
#[poise::command(
    slash_command,
    category = "Moderation",
    default_member_permissions = "MANAGE_GUILD",
    prefix_command,
    track_edits
)]
pub async fn reason(
    ctx: Context<'_>,
    #[description = "The case to assign a reason."] case: u32,
    #[description = "The reasoning for the case."]
    #[rest]
    reason: String,
) -> Result<(), Error> {
    let database = &ctx.data().database();
    let guild = ctx.guild_id().unwrap();
    let guild_id = guild.get();
    let mut db = Valeriyya::get_database(database, guild_id).await?;

    let case_find = match db.cases.iter().find(|c| c.id == case) {
        Some(c) => c.clone(),
        None => {
            ctx.send(
                Valeriyya::reply(format!("Case with the id: {} doesn't exist", case))
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
    };

    db = db.update_case(
        case,
        CaseUpdateAction::Reason,
        CaseUpdateValue {
            reason: Some(reason.clone()),
            reference: None,
        },
    );

    ctx.send(Valeriyya::reply(format!("Updated case with the id: {case}")).ephemeral(true))
        .await?;

    if let Some(logs) = &db.channels.logs {
        let channel = ChannelId::new(logs.parse::<u64>().unwrap());
        if case_find.message.is_some() {
            let mut log_channel_msg = channel
                .widen()
                .message(
                    ctx.serenity_context(),
                    MessageId::new(
                        case_find
                            .message
                            .as_deref()
                            .unwrap()
                            .parse::<u64>()
                            .unwrap(),
                    ),
                )
                .await?;
            let staff_user_cache = UserId::new(case_find.staff_id.parse::<u64>().unwrap())
                .to_user(ctx.serenity_context())
                .await?;
            let (staff_name, staff_id, staff_face) = (
                &staff_user_cache.name,
                staff_user_cache.id,
                staff_user_cache.face(),
            );
            let target_user = UserId::new(case_find.target_id.parse::<u64>().unwrap())
                .to_user(ctx.serenity_context())
                .await?
                .name;
            let icon_url = ctx.guild().unwrap().icon_url().unwrap_or(String::from(""));

            let embed = Valeriyya::embed()
                .timestamp(Timestamp::from(
                    &Timestamp::from_unix_timestamp(case_find.date).unwrap(),
                ))
                .author(
                    Valeriyya::reply_author(format!("{} ({})", staff_name, staff_id))
                        .icon_url(staff_face),
                )
                .thumbnail(icon_url)
                .footer(Valeriyya::reply_footer(format!("Case {}", case_find.id)))
                .description(format!(
                    "Member: `{}`\nAction: `{:?}`\nReason: `{}`\n{}",
                    target_user,
                    case_find.action,
                    reason,
                    match case_find.action {
                        ActionTypes::Mute => format!(
                            "Expiration: {}",
                            Valeriyya::time_format(case_find.expiration.unwrap().to_string())
                        ),
                        _ => "".to_string(),
                    }
                ));

            log_channel_msg
                .edit(ctx.serenity_context(), Valeriyya::msg_edit().embed(embed))
                .await?;
        };
    }
    db.execute(database).await;
    Ok(())
}
