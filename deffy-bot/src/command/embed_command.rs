use std::vec;

use anyhow::Error;
use deffy_bot_macro::command;
use serde::Deserialize;
use serenity::{
    all::{
        ButtonStyle, CommandDataOption, CommandInteraction, Context, CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, EditMessage
    },
    async_trait,
};

use crate::command::system::manager::{CommandHandler, CommandInfo};

#[derive(Deserialize)]
struct EmbedJson {
    title: String,
    description: String,
    color: i32,
    fields: Vec<EmbedField>,
}

#[derive(Deserialize)]
struct EmbedWrapper {
    components: Vec<EmbedComponent>,
}

#[derive(Deserialize)]
struct EmbedComponent {
    #[serde(rename = "type")]
    component_type: i32,
    components: Vec<Component>,
}

#[derive(Deserialize)]
struct Component {
    #[serde(rename = "id")]
    custom_id: String,
    #[serde(rename = "type")]
    c_type: i32,
    style: i32,
    label: String,
}

#[derive(Deserialize)]
struct EmbedField {
    name: String,
    value: String,
    inline: Option<bool>,
}

#[derive(Deserialize)]
enum MyComponentType {
    #[serde(rename = "1")]
    ActionRow,
    #[serde(rename = "2")]
    Button,
}

#[command(cmd = embed, cooldown = 0)]
pub struct EmbedCommand;

#[async_trait]
impl CommandHandler for EmbedCommand {
    async fn execute(&self, ctx: Context, interaction: CommandInteraction) -> Result<(), Error> {
        if let Some(input) = interaction.data.options.get(0) {
            match input.value.as_str() {
                Some("ctembed") => {
                    if let Some(content) = interaction.data.options.get(1) {
                        let rsp = generate_embed(content.clone())?;

                        interaction
                            .channel_id
                            .send_message(&ctx.http, rsp.0)
                            .await?;
                    }
                }
                Some("editembed") => {
                    if let Some(content) = interaction.data.options.get(1) {
                        let rsp = generate_embed(content.clone())?;

                        let message_id_option = interaction
                            .data
                            .options
                            .get(2)
                            .and_then(|opt| opt.value.as_str())
                            .and_then(|s| s.parse::<u64>().ok());

                        if let Some(message_id) = message_id_option {
                            use serenity::all::MessageId;
                            interaction
                                .channel_id
                                .edit_message(&ctx.http, MessageId::from(message_id), rsp.1)
                                .await?;
                        } else {
                            return Err(anyhow::anyhow!("Missing Input"));
                        }
                    }
                }

                _ => {}
            }
        }

        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!("Success!"))
                .ephemeral(true),
        );

        interaction.create_response(ctx.http, response).await?;

        Ok(())
    }

    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("A Embed Creation Command")
            .add_option(
                CreateCommandOption::new(
                    serenity::all::CommandOptionType::String,
                    "action",
                    "An input string for create or edit embed",
                )
                .required(true)
                .add_string_choice("Create Content", "ctembed")
                .add_string_choice("Edit Embed", "editembed"),
            )
            .add_option(
                CreateCommandOption::new(
                    serenity::all::CommandOptionType::String,
                    "jsoncontent",
                    "a json format for create embed",
                )
                .required(true),
            )
            .add_option(CreateCommandOption::new(
                serenity::all::CommandOptionType::String,
                "messageid",
                "A MessageId for edit embed",
            ))
    }
}

fn generate_embed(content: CommandDataOption) -> Result<(CreateMessage, EditMessage), Error> {
    if let Some(json_str) = content.value.as_str() {
        let jcontent: EmbedJson = serde_json::from_str(json_str)?;
        let embed = CreateEmbed::new()
            .title(jcontent.title)
            .description(jcontent.description)
            .color(jcontent.color)
            .fields(
                jcontent
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), f.value.clone(), f.inline.unwrap_or(false)))
                    .collect::<Vec<_>>(),
            );

        let components = parse_components_from_json(json_str).unwrap_or(vec![]);

        let create_message = CreateMessage::new()
            .add_embed(embed.clone())
            .components(components.clone());

        let edit_message = EditMessage::new().add_embed(embed).components(components);

        Ok((create_message, edit_message))
    } else {
        Err(anyhow::anyhow!("Missing Content Value"))
    }
}

fn button_style_from_i32(value: i32) -> ButtonStyle {
    match value {
        1 => ButtonStyle::Primary,
        2 => ButtonStyle::Secondary,
        3 => ButtonStyle::Success,
        4 => ButtonStyle::Danger,
        _ => ButtonStyle::Primary, // fallback
    }
}

fn parse_components_from_json(json_str: &str) -> Result<Vec<CreateActionRow>, serde_json::Error> {
    let parsed: EmbedWrapper = serde_json::from_str(json_str)?;
    let mut action_rows = Vec::new();

    for row in parsed.components {
        if row.component_type != 1 {
            continue; // ไม่ใช่ ActionRow
        }

        let mut buttons = Vec::new();

        for comp in row.components {
            if comp.c_type != 2 {
                continue; // ไม่ใช่ปุ่ม
            }

            let button = CreateButton::new(comp.custom_id) // ใส่ custom_id จริงตามการใช้งาน
                .label(&comp.label)
                .style(button_style_from_i32(comp.style));

            buttons.push(button);
        }

        action_rows.push(CreateActionRow::Buttons(buttons));
    }

    Ok(action_rows)
}
