use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use colored::Colorize;
use ctrlc;
use dialoguer::{
    self, console::{Style, Term}, theme::ColorfulTheme,
    FuzzySelect, Select, Input, Confirm
};
use directories::ProjectDirs;

mod config;
use config::{ConfigCommandItem, ConfigCommandItemState, ConfigItem, ConfigManager};

#[derive(Parser)]
#[command(version)]
#[command(about = "Easily switch your config items in terminal", long_about = None)]
struct Cli {
    /// Add a config item
    #[arg(short, long, name = "ADD_NAME", conflicts_with_all = vec!["REMOVE_NAME", "LIST_NAME", "ITEM"])]
    add: Option<String>,

    /// Remove a config item
    #[arg(short, long, name = "REMOVE_NAME", conflicts_with_all = vec!["LIST_NAME", "ITEM"])]
    remove: Option<String>,

    /// List all config items
    #[arg(short, long, name = "LIST_NAME", conflicts_with_all = vec!["ITEM"])]
    list: bool,

    /// Name of the config item to switch, fuzzy
    #[arg(name = "ITEM")]
    item: Option<String>,

    /// Debug mode
    #[arg(short, long)]
    debug: bool
}

fn main_wrapper(cli: Cli) -> Result<()> {
    let project_dirs = ProjectDirs::from("top", "IceLava", "switchy")
        .context("Failed to get config dir")?;
    let mut cm = ConfigManager::new(project_dirs.config_dir());
    cm.read()?;

    let mut colorful_theme = ColorfulTheme::default();
    colorful_theme.prompt_style = Style::new().for_stderr().cyan();

    if let Some(name) = cli.add {
        if cm.config.items.iter().any(|item| item.get_name() == name) {
            bail!("Config item {} already exists", name.cyan());
        }

        println!("Adding config item {}", name.cyan());

        let item_type = Select::with_theme(&colorful_theme)
            .with_prompt("The type of the item")
            .default(0)
            .items(&vec![
                "Command item"
            ])
            .interact()?;

        let item = match item_type {
            0 => {
                let mut states: Vec<ConfigCommandItemState> = vec![];
                loop {
                    if states.is_empty() {
                        println!("Adding default state");
                    }
                    else if ! Confirm::with_theme(&colorful_theme)
                        .with_prompt("To add another state?")
                        .interact()?
                    {
                        break;
                    }
                    
                    let name = Input::<String>::with_theme(&colorful_theme)
                        .with_prompt("State name")
                        .interact_text()?
                        .trim()
                        .to_string();

                    if name.is_empty() {
                        bail!("State name is empty");
                    }
                    if states.iter().any(|state| state.name == name) {
                        bail!("State name '{}' is used", name);
                    }

                    let command = Input::<String>::with_theme(&colorful_theme)
                        .with_prompt("State command")
                        .interact_text()?
                        .trim()
                        .to_string();

                    states.push(ConfigCommandItemState { name, command });
                }
                
                ConfigItem::CommandItem(ConfigCommandItem {
                    name,
                    current: states[0].name.clone(),
                    states
                })
            },
            _ => unreachable!()
        };

        cm.config.items.push(item);
        cm.write()?;
    }

    else if let Some(name) = cli.remove {
        if let Some(index) = cm.config.items.iter().position(|item| item.get_name() == name) {
            println!("Removing config item {}", name.cyan());
            cm.config.items.swap_remove(index);
            cm.write()?;
        }
        else {
            bail!("Config item {} doesn't exist", name.cyan());
        }
    }

    else if cli.list {
        let len = cm.config.items.len();
        if len == 0 {
            println!("No config items yet.");
        }
        else {
            println!(
                "Listing all {} config item(s):\n\n{}",
                len,
                cm.config.items
                    .iter()
                    .map(String::from)
                    .collect::<Vec<_>>()
                    .join("\n\n")
            );
        }
    }

    else {
        if cm.config.items.is_empty() {
            Err(anyhow!("No config items yet. Use `--add` to add one. Use `--help` for more information"))?;
        }
        else {
            let item_names: Vec<&str> = cm.config.items
                .iter()
                .map(|item| item.get_name())
                .collect();

            let item_name = cli.item.unwrap_or("".to_string());
            let item_index = item_names
                .iter()
                .position(|name| *name == item_name)
                .map_or_else(
                    || FuzzySelect::with_theme(&colorful_theme)
                        .with_initial_text(item_name)
                        .default(0)
                        .items(&item_names)
                        .interact(),
                    Ok
                )?;

            let item = &mut cm.config.items[item_index];

            let state_names = item.get_state_names();
            let current_state = item.get_current_state();
            let current_state_index = state_names.iter().position(|name| *name == current_state).unwrap();
            let new_state_index = FuzzySelect::with_theme(&colorful_theme)
                .default(current_state_index)
                .items(&state_names)
                .interact()?;
            let new_state = state_names[new_state_index].to_string();

            if  new_state_index != current_state_index ||
                Confirm::with_theme(&colorful_theme)
                    .with_prompt(format!("{} is the current state. Reset?", new_state.yellow()))
                    .interact()?
            {
                item.set_current_state(new_state)?;
                cm.write()?;
            }
        }
    }

    Ok(())
}

fn main() {
    ctrlc::set_handler(|| {
        Term::stdout().show_cursor().unwrap();
    }).unwrap();

    let cli = Cli::parse();
    let debug_mode = cli.debug;

    if let Err(err) = main_wrapper(cli) {
        if err.is::<dialoguer::Error>() {
            let dialoguer::Error::IO(err) = err.downcast_ref::<dialoguer::Error>().unwrap();
            if err.kind() == std::io::ErrorKind::Interrupted {
                return;
            }
        }

        if debug_mode {
            eprintln!("{:#?}", err);
        }
        else {
            eprintln!("{}", err.to_string().red());
        }
    }
}   
