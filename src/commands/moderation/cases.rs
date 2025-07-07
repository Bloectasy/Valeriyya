use poise::serenity_prelude::{CreateEmbed, Member, Timestamp, UserId};

use crate::{
    structs::{ActionTypes, Case},
    utils::{get_guild_member, Valeriyya},
    Context, Error,
};

#[derive(poise::ChoiceParameter)]
pub enum OptionChoices {
    #[name = "show"]
    Show,
    #[name = "delete"]
    Delete,
}

#[doc = "Shows or deletes a case from guilds actions."]
#[poise::command(
    slash_command,
    category = "Moderation",
    default_member_permissions = "MANAGE_GUILD",
    prefix_command,
    track_edits
)]
pub async fn cases(
    ctx: Context<'_>,
    #[description = "What to do with the case."] option: OptionChoices,
    #[description = "The id of the case."] id: u32,
) -> Result<(), Error> {
    let database = &ctx.data().database();

    let guild_id = ctx.guild_id().unwrap().get();

    let mut db = Valeriyya::get_database(database, guild_id).await?;
    let staff = get_guild_member(ctx).await?.unwrap();

    if let OptionChoices::Show = option {
        let case = db.cases.iter().find(|c| c.id == id);

        if case.is_none() {
            ctx.send(
                Valeriyya::reply(format!("Can't find a case with the id: {}", id)).ephemeral(true),
            )
            .await?;
            return Ok(());
        }

        let case = case.unwrap();
        let target_user = UserId::new(case.target_id.parse::<u64>().unwrap())
            .to_user(ctx.serenity_context())
            .await?
            .tag();

        ctx.send(Valeriyya::reply_default().embed(create_embed(ctx, staff, case, target_user)))
            .await?;
    } else if let OptionChoices::Delete = option {
        let case = db.cases.iter().find(|c| c.id == id);

        if case.is_none() {
            ctx.send(
                Valeriyya::reply(format!("Can't find a case with the id: {}", id)).ephemeral(true),
            )
            .await?;
            return Ok(());
        }

        let case = case.unwrap();

        let index = db
            .cases
            .iter()
            .position(|indexc| indexc.id == case.id)
            .unwrap();

        db = db.delete_cases(index);

        ctx.send(
            Valeriyya::reply_default().embed(
                Valeriyya::embed().author(
                    Valeriyya::reply_author(format!("{} ({})", staff.user.tag(), staff.user.id))
                        .icon_url(staff.user.face()),
                ),
            ),
        )
        .await?;
    }
    db.execute(database).await;
    Ok(())
}

fn create_embed<'a>(
    ctx: Context<'_>,
    staff: Member,
    case: &Case,
    target_user: String,
) -> CreateEmbed<'a> {
    let expiration_text = case.expiration.map(|exp| format!("<t:{}:R>", exp));

    let description = match (&case.action, &expiration_text, case.reference) {
        (ActionTypes::Mute, Some(exp), Some(reference)) => {
            format!(
                "Member: `{}`\nAction: `{:?}`\nReason: `{}`\nExpiration: {}\nReference: `{}`",
                target_user, case.action, case.reason, exp, reference
            )
        }
        (ActionTypes::Mute, Some(exp), None) => {
            format!(
                "Member: `{}`\nAction: `{:?}`\nReason: `{}`\nExpiration: {}",
                target_user, case.action, case.reason, exp
            )
        }
        (_, _, Some(reference)) => {
            format!(
                "Member: `{}`\nAction: `{:?}`\nReason: `{}`\nReference: `{}`",
                target_user, case.action, case.reason, reference
            )
        }
        _ => {
            format!(
                "Member: `{}`\nAction: `{:?}`\nReason: `{}`",
                target_user, case.action, case.reason
            )
        }
    };

    Valeriyya::embed()
        .author(
            Valeriyya::reply_author(format!("{} ({})", staff.user.tag(), staff.user.id))
                .icon_url(staff.user.face()),
        )
        .thumbnail(ctx.guild().unwrap().icon_url().unwrap())
        .timestamp(Timestamp::from(
            &Timestamp::from_unix_timestamp(case.date).unwrap(),
        ))
        .footer(Valeriyya::reply_footer(format!("Case {}", case.id)))
        .description(description)
}
