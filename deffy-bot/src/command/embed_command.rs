use std::{string, vec};

use serde::Deserialize;
use serenity::{
    all::{
        CommandInteraction, Context, CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, ModelError
    },
    async_trait, Error,
};

use crate::command::command_registry::{CommandHandler, CommandInfo};

#[derive(Deserialize)]
struct EmbedJson {
    title: String,
    description: String,
    color: i32,
    fields: Vec<EmbedField>,
    
}

#[derive(Deserialize)]
struct EmbedComponent {
    componentType: i32,
    component: Vec<Component>,
}

#[derive(Deserialize)]
struct Component {
    cType: i32,
    style: i32,
    lable: String,
}

#[derive(Deserialize)]
struct EmbedField {
    name: String,
    value: String,
    inline: Option<bool>,
}

pub struct EmbedCommand;

#[async_trait]
impl CommandHandler for EmbedCommand {
    async fn execute(
        &self,
        ctx: Context,
        interaction: CommandInteraction,
    ) -> Result<(), Error> {
        if let Some(input) = interaction.data.options.get(0) {
            match input.value.as_str() {
                Some("ctembed") => {
                    if let Some(content) = interaction.data.options.get(1) {
                        // Try to parse the JSON content into EmbedJson struct

                        if let Some(json_str) = content.value.as_str() {
                            let jcontent: EmbedJson = match serde_json::from_str(json_str) {
                                Ok(val) => val,
                                Err(e) => {
                                    tracing::error!("Failed to parse embed JSON: {:?}", e);
                                    return Err(serenity::Error::Json(e));
                                }
                            };
                            let embed = CreateEmbed::new()
                                .title(jcontent.title)
                                .description(jcontent.description)
                                .color(jcontent.color)
                                .fields(
                                    jcontent
                                        .fields
                                        .iter()
                                        .map(|f| {
                                            (
                                                f.name.clone(),
                                                f.value.clone(),
                                                f.inline.unwrap_or(false),
                                            )
                                        })
                                        .collect::<Vec<_>>(),
                                );

                                let mut comps: Vec<CreateActionRow> = Vec::new();

                                comps.push(CreateActionRow::Buttons(vec![CreateButton::new("myBtn")]));

                            let messagersp = CreateMessage::new().add_embed(embed).components(comps);

                            if let Err(e) = interaction.channel_id.send_message(&ctx, messagersp).await {
                                tracing::error!("{:?}",e)
                            }
                        }
                    }
                }
                Some("ccompoembed") => {}
                _ => {}
            }
        }

        let content = format!(
            "Hello, {} This is a test command response.",
            interaction.user.name
        );

        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(content)
                .ephemeral(true),
        );

        interaction.create_response(ctx, response).await
    }
    fn register(&self) -> CreateCommand {
        CreateCommand::new(self.name())
            .description("A test command")
            .add_option(
                CreateCommandOption::new(
                    serenity::all::CommandOptionType::String,
                    "action",
                    "An input string for create or edit embed",
                )
                .required(true)
                .add_string_choice("Create Content", "ctembed")
                .add_string_choice("Create Component", "ccompoembed"),
            )
            .add_option(
                CreateCommandOption::new(
                    serenity::all::CommandOptionType::String,
                    "jsoncontent",
                    "jsonembed",
                )
                .required(true),
            )
    }
}

impl CommandInfo for EmbedCommand {
    fn name(&self) -> &'static str {
        "embed"
    }
}
