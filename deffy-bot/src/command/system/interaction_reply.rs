use serenity::{
    all::CreateEmbed, builder::{CreateInteractionResponse, CreateInteractionResponseMessage}, client::Context, model::application::CommandInteraction, Error
};

pub trait InteractionExt {
    async fn reply(&self, ctx: &Context, content: impl Into<String>, ephemeral: bool) -> Result<(), Error>;
    async fn reply_embed(&self, ctx: &Context, embed: CreateEmbed, ephemeral: bool) -> Result<(), Error>;
}

impl InteractionExt for CommandInteraction {
    async fn reply(&self, ctx: &Context, content: impl Into<String>, ephemeral: bool) -> Result<(), Error> {
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(content.into())
                .ephemeral(ephemeral),
        );
        self.create_response(&ctx.http, response).await
    }

    async fn reply_embed(&self, ctx: &Context, embed: CreateEmbed, ephemeral: bool) -> Result<(), Error> {
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .embed(embed)
                .ephemeral(ephemeral),
        );
        self.create_response(&ctx.http, response).await
    }
}

impl InteractionExt for serenity::model::prelude::Interaction {
    async fn reply(&self, ctx: &Context, content: impl Into<String>, ephemeral: bool) -> Result<(), Error> {
        if let Some(interaction) = self.as_command() {
            interaction.reply(ctx, content, ephemeral).await
        } else {
            Err(Error::Other("Interaction is not a command".into()))
        }
    }

    async fn reply_embed(&self, ctx: &Context, embed: CreateEmbed, ephemeral: bool) -> Result<(), Error> {
        if let Some(interaction) = self.as_command() {
            interaction.reply_embed(ctx, embed, ephemeral).await
        } else {
            Err(Error::Other("Interaction is not a command".into()))
        }
    }
    
}