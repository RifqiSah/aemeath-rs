use poise::serenity_prelude as serenity;

async fn reply<U, E>(
  ctx: poise::Context<'_, U, E>,
  content: impl Into<String>,
) -> Result<(), serenity::Error> {
  match ctx {
    poise::Context::Application(ctx) => {
      ctx.framework.serenity_context.http.create_interaction_response(
        ctx.interaction.id,
        &ctx.interaction.token,
        &serenity::CreateInteractionResponse::Message(
          serenity::CreateInteractionResponseMessage::new()
            .content(content.into())
            .ephemeral(true),
        ),
        vec![],
      ).await?;
    }
    poise::Context::Prefix(ctx) => {
      ctx.msg.channel_id.say(&ctx.framework.serenity_context.http, content.into()).await?;
    }
  }

  Ok(())
}

pub async fn on_error<U, E: std::fmt::Display + std::fmt::Debug>(
  error: poise::FrameworkError<'_, U, E>,
) -> Result<(), serenity::Error> {
  match error {
    poise::FrameworkError::Command { ctx, error, .. } => {
      let error = error.to_string();
      let error_message = format!("An error occured in a command: {}", error);

      tracing::error!(error_message);
      reply(ctx, error_message).await?;
    }
    poise::FrameworkError::CommandPanic { ctx, payload: _, .. } => {
      let error_message = format!("An error occured while executing command!");

      tracing::error!(error_message);
      reply(ctx, error_message).await?;
    }
    poise::FrameworkError::SubcommandRequired { ctx: _ } => {
      // TODO
    },
    poise::FrameworkError::ArgumentParse { error: _, input: _, ctx: _, .. } => {
      // TODO
    },
    poise::FrameworkError::CommandStructureMismatch { description: _, ctx: _, .. } => {
      // TODO
    },
    poise::FrameworkError::CommandCheckFailed { error: _, ctx: _, .. } => {
      // TODO
    },
    poise::FrameworkError::DynamicPrefix { error: _, ctx: _, msg: _, .. } => {
      // TODO
    },
    poise::FrameworkError::CooldownHit { remaining_cooldown, ctx, .. } => {
      let error_message = format!(
        "Rate limiting is in progress. Please try again in {} seconds.",
        remaining_cooldown.as_secs()
      );

      tracing::error!(error_message);
      reply(ctx, error_message).await?;
    }
    poise::FrameworkError::MissingBotPermissions { missing_permissions, ctx, .. } => {
      let error_message = format!(
        "Failed to execute command. Please check bot permissions: {}",
        missing_permissions,
      );

      tracing::error!(error_message);
      reply(ctx, error_message).await?;
    }
    poise::FrameworkError::MissingUserPermissions { missing_permissions: _, ctx: _, .. } => {
      // TODO
    },
    poise::FrameworkError::PermissionFetchFailed { ctx: _ , .. } => {
      // TODO
    },
    poise::FrameworkError::NotAnOwner { ctx, .. } => {
      let error_message = "You do not have permission to execute the command";
      reply(ctx, error_message).await?;
    }
    poise::FrameworkError::GuildOnly { ctx, .. } => {
      let error_message = "This command can only be executed on the server";
      reply(ctx, error_message).await?;
    }
    poise::FrameworkError::DmOnly { ctx, .. } => {
      let error_message = "This command can only be executed by DM";
      reply(ctx, error_message).await?;
    }
    poise::FrameworkError::NsfwOnly { ctx, .. } => {
      let error_message = "This command can only be executed in the NSFW channel";
      reply(ctx, error_message).await?;
    }
    poise::FrameworkError::UnknownCommand { msg, trigger, .. } => {
      tracing::warn!(
        "Didn't recognize message from trigger: {:?} -> {:?}", trigger, msg,
      );
    }
    poise::FrameworkError::NonCommandMessage { error: _, framework: _, msg: _, .. } => {
      // TODO
    },
    poise::FrameworkError::UnknownInteraction { interaction, .. } => {
      tracing::warn!("Received unknown interaction \"{}\"", interaction.data.name);
    }
    poise::FrameworkError::__NonExhaustive(unreachable) => match unreachable {},
  }

  Ok(())
}