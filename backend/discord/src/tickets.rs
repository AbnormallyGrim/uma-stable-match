//! An application's ticket: a private channel in the club's server for the applicant and the
//! club's leader, with the applicant's details and buttons to answer.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::application_store::Application;
use crate::apply::Applicant;
use crate::bot::Discord;
use crate::error::Error;
use crate::store::ClubRecord;
use crate::ticket_name::random_number;

const VIEW_CHANNEL: u64 = 1 << 10;
const SEND_MESSAGES: u64 = 1 << 11;
const EMBED_LINKS: u64 = 1 << 14;
const READ_MESSAGE_HISTORY: u64 = 1 << 16;
const MANAGE_CHANNELS: u64 = 1 << 4;

/// What the applicant and the leader can do in the ticket.
const PEOPLE: u64 = VIEW_CHANNEL | SEND_MESSAGES | READ_MESSAGE_HISTORY;

/// What the bot needs in the ticket, which it lacks once the server's members can't see it.
const BOT: u64 = PEOPLE | EMBED_LINKS | MANAGE_CHANNELS;

const BLURPLE: u32 = 0x5865f2;

#[derive(Deserialize)]
struct Created {
    id: String,
}

/// Opens the ticket channel for `application` and posts its details. Returns the channel and the
/// message with the answer buttons.
pub(crate) async fn open(
    discord: &Discord,
    record: &ClubRecord,
    application: &Application,
    applicant: &Applicant,
) -> Result<(u64, u64), Error> {
    let guild = record.guild_id;
    let details = &application.details;
    let channel = match create_channel(discord, record, application, record.leader).await {
        Ok(channel) => channel,
        // The leader may have left the server since they set the club up.
        Err(Error::Api { status, .. }) if status.as_u16() == 400 && record.leader.is_some() => {
            create_channel(discord, record, application, None).await?
        }
        Err(err) => return Err(err),
    };
    let channel_id = id(&channel.id)?;
    let message: Created = discord
        .rest()
        .post(
            &format!("/channels/{channel_id}/messages"),
            &message(
                application,
                applicant,
                record.leader,
                discord.config().public_url.as_str(),
            ),
        )
        .await?;
    tracing::info!(guild, club = details.club, "opened an application ticket");
    Ok((channel_id, id(&message.id)?))
}

async fn create_channel(
    discord: &Discord,
    record: &ClubRecord,
    application: &Application,
    leader: Option<u64>,
) -> Result<Created, Error> {
    let guild = record.guild_id;
    let mut overwrites = vec![
        // Nobody else sees it.
        json!({ "id": guild.to_string(), "type": 0, "allow": "0", "deny": VIEW_CHANNEL.to_string() }),
        json!({
            "id": discord.config().app_id.get().to_string(),
            "type": 1,
            "allow": BOT.to_string(),
            "deny": "0",
        }),
        member(application.details.applicant),
    ];
    if let Some(leader) = leader {
        overwrites.push(member(leader));
    }
    let mut body = json!({
        "name": record.ticket_name.channel_name(
            &application.details.club_name,
            &application.details.trainer_name,
            random_number(),
        ),
        "type": 0,
        "topic": format!("Application to {}", application.details.club_name),
        "permission_overwrites": overwrites,
    });
    if let Some(category) = record.category {
        body["parent_id"] = json!(category.to_string());
    }
    discord
        .rest()
        .post(&format!("/guilds/{guild}/channels"), &body)
        .await
}

fn member(user: u64) -> Value {
    json!({ "id": user.to_string(), "type": 1, "allow": PEOPLE.to_string(), "deny": "0" })
}

fn id(value: &str) -> Result<u64, Error> {
    value
        .parse()
        .map_err(|_| Error::Decode(serde::de::Error::custom("an ID isn't a whole number")))
}

/// A whole number with thousands separators.
fn grouped(number: i64) -> String {
    let digits = number.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if number < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// The ticket's opening message. `site` is the site's address, for the compare link.
fn message(
    application: &Application,
    applicant: &Applicant,
    leader: Option<u64>,
    site: &str,
) -> Value {
    let details = &application.details;
    let profile = format!("https://uma.moe/profile/{}", details.trainer);
    let compare = format!(
        "{site}/applications/club/{}/compare?trainers={}",
        details.club, details.trainer
    );
    let mut fields = vec![
        json!({
            "name": "Trainer",
            "value": format!("[{}]({profile})\nID `{}`", details.trainer_name, details.trainer),
            "inline": true,
        }),
        json!({ "name": "Applicant", "value": format!("<@{}>", details.applicant), "inline": true }),
        json!({
            "name": "Current club",
            "value": applicant.club_name.clone().unwrap_or_else(|| "None".to_owned()),
            "inline": true,
        }),
    ];
    if let Some(pace) = applicant.pace_per_day {
        fields.push(json!({
            "name": "Pace",
            "value": format!("{}/day", grouped(pace.round() as i64)),
            "inline": true,
        }));
    }
    if let Some(fans) = applicant.fans_this_month {
        fields.push(json!({
            "name": "Fans this month",
            "value": grouped(fans),
            "inline": true,
        }));
    }
    if let Some(alltime) = &applicant.alltime {
        fields.push(json!({
            "name": "Total fans",
            "value": grouped(alltime.total_fans),
            "inline": true,
        }));
        for (name, average) in [
            ("Average per day", alltime.avg_day),
            ("Average per week", alltime.avg_week),
            ("Average per month", alltime.avg_month),
        ] {
            if let Some(average) = average {
                fields.push(json!({
                    "name": name,
                    "value": grouped(average.round() as i64),
                    "inline": true,
                }));
            }
        }
        if let Some(rank) = alltime.rank {
            fields.push(json!({
                "name": "Fans rank",
                "value": format!("#{}", grouped(rank)),
                "inline": true,
            }));
        }
        fields.push(json!({
            "name": "Days tracked",
            "value": grouped(alltime.active_days),
            "inline": true,
        }));
    }
    let mut users = vec![details.applicant.to_string()];
    let mut content = format!("New application to **{}**.", details.club_name);
    if let Some(leader) = leader {
        content = format!("<@{leader}> {content}");
        users.push(leader.to_string());
    }
    json!({
        "content": content,
        "allowed_mentions": { "users": users },
        "embeds": [{
            "title": format!("Application from {}", details.trainer_name),
            "url": profile,
            "color": BLURPLE,
            "fields": fields,
        }],
        "components": [{
            "type": 1,
            "components": [
                {
                    "type": 2,
                    "style": 3,
                    "label": "Accept",
                    "custom_id": format!("apply:accept:{}", application.id),
                },
                {
                    "type": 2,
                    "style": 4,
                    "label": "Decline",
                    "custom_id": format!("apply:decline:{}", application.id),
                },
                { "type": 2, "style": 5, "label": "Compare", "url": compare },
            ],
        }],
    })
}

#[cfg(test)]
mod tests;
