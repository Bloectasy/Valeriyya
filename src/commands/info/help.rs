use std::collections::HashMap;
use crate::{Context, Error};

#[poise::command(prefix_command, guild_only, hide_in_help, track_edits)]
pub async fn help(ctx: Context<'_>, command: Option<String>) -> Result<(), Error> {
    let commands = &ctx.framework().options().commands;
    
    // commands.first().expect("dasdsa")
    if let Some(command_name) = command {
        if let Some(cmd) = commands.iter().find(|c| c.name == command_name) {
            let mut help_message = format!("# {}\n", cmd.name); // Command name as h1

            help_message.push_str(&format!("\nAliases: {}\n", cmd.aliases.join(", ")));

            // help_message.push_str(&format!("Cooldown: {} seconds\n", cmd.cooldowns.));

            help_message.push_str(&format!(
                "Permissions: {}\n",
                cmd.required_permissions.iter().map(|p| p.to_string()).collect::<Vec<String>>().join(", ")
            ));

            // Add ephemeral info
            help_message.push_str(&format!("Ephemeral: {}\n", cmd.ephemeral));

            // Add parameters
            if !cmd.parameters.is_empty() {
                help_message.push_str("\nParameters:\n");
                for param in &cmd.parameters {
                    help_message.push_str(&format!(
                        "> **{}**: {}\n", 
                        param.name,
                        param.description.clone().unwrap_or("No description available.".to_string().into())
                    ));
                }
            } else {
                help_message.push_str("\nNo parameters.\n");
            }

            // Add the description
            help_message.push_str(&format!("\nDescription: {}\n", cmd.description.clone().unwrap_or("No description available.".to_string().into())));

            // Send the detailed help message for the command
            ctx.channel_id().say(ctx.http(), help_message).await?;
        } else {
            // If the command is not found, inform the user
            ctx.channel_id().say(ctx.http(), "Command not found.").await?;
        }
    } else {
        let mut categories: HashMap<String, Vec<String>> = HashMap::new();
        
        for command in commands {
            let category = command.category.clone().unwrap_or_else(|| "Uncategorized".into());
            categories.entry(category.to_string()).or_insert_with(Vec::new).push(command.name.to_string());
        }

        let mut help_message = "Here are the available commands:\n".to_string();
        
        for (category, commands) in categories {
            
            help_message.push_str(&format!("\n# {}\n", category));
            for command in commands {
                
                help_message.push_str(&format!("> {}\n", command));
            }
        }

        ctx.channel_id().say(ctx.http(), help_message).await?;
    }

    Ok(())
}
