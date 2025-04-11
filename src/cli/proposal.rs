use clap::{arg, Command};
use std::error::Error;
use crate::vm::VM;
use crate::compiler::parse_dsl;
use crate::storage::StorageBackend;
use crate::identity::AuthContext;

pub fn proposal_command() -> Command {
    Command::new("proposal")
        .about("Manage proposal lifecycle")
        .subcommand(create_command())
        .subcommand(attach_command())
        .subcommand(vote_command())
}

fn create_command() -> Command {
    Command::new("create")
        .about("Create a new proposal")
        .arg(
            arg!(<id> "Unique proposal identifier")
                .required(true)
        )
        .arg(
            arg!(--title <STRING> "Proposal title")
                .default_value("Untitled Proposal")
        )
        .arg(
            arg!(--author <STRING> "Proposal author")
                .default_value("anonymous")
        )
        .arg(
            arg!(--quorum <FLOAT> "Required quorum percentage")
                .default_value("0.6")
                .value_parser(clap::value_parser!(f64))
        )
        .arg(
            arg!(--threshold <FLOAT> "Required approval threshold")
                .default_value("0.5")
                .value_parser(clap::value_parser!(f64))
        )
}

fn attach_command() -> Command {
    Command::new("attach")
        .about("Attach a document section to a proposal")
        .arg(
            arg!(<id> "Proposal identifier")
                .required(true)
        )
        .arg(
            arg!(<section> "Document section name (e.g. summary, rationale)")
                .required(true)
        )
        .arg(
            arg!(<text> "Section text content")
                .required(true)
        )
}

fn vote_command() -> Command {
    Command::new("vote")
        .about("Cast a ranked vote on a proposal")
        .arg(
            arg!(<id> "Proposal identifier")
                .required(true)
        )
        .arg(
            arg!(--ranked <RANKS> "Ranked choices (space separated integers)")
                .required(true)
                .num_args(1..)
                .value_parser(clap::value_parser!(u32))
        )
        .arg(
            arg!(--identity <STRING> "Identity to sign the vote with")
        )
}

pub fn handle_proposal_command(
    matches: &clap::ArgMatches,
    vm: &mut VM,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>> {
    match matches.subcommand() {
        Some(("create", create_matches)) => {
            let id = create_matches.get_one::<String>("id").unwrap();
            let title = create_matches.get_one::<String>("title").unwrap();
            let author = create_matches.get_one::<String>("author").unwrap();
            let quorum = create_matches.get_one::<f64>("quorum").unwrap();
            let threshold = create_matches.get_one::<f64>("threshold").unwrap();

            let dsl = format!(
                r#"proposal_lifecycle "{}" quorum={} threshold={} title="{}" author="{}" {{
                    emit "Proposal created"
                }}"#,
                id, quorum, threshold, title, author
            );

            let ops = parse_dsl(&dsl)?;
            vm.execute(&ops)?;
        }

        Some(("attach", attach_matches)) => {
            let id = attach_matches.get_one::<String>("id").unwrap();
            let section = attach_matches.get_one::<String>("section").unwrap();
            let text = attach_matches.get_one::<String>("text").unwrap();

            let dsl = format!(
                r#"storep "proposals/{}/docs/{}" "{}""#,
                id, section, text
            );

            let ops = parse_dsl(&dsl)?;
            vm.execute(&ops)?;
        }

        Some(("vote", vote_matches)) => {
            let id = vote_matches.get_one::<String>("id").unwrap();
            let ranks: Vec<u32> = vote_matches
                .get_many::<u32>("ranked")
                .unwrap()
                .copied()
                .collect();

            let ranks_str = ranks
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(" ");

            let dsl = format!(r#"rankedvote "{}" {}"#, id, ranks_str);

            let ops = parse_dsl(&dsl)?;
            vm.execute(&ops)?;
        }

        _ => unreachable!(),
    }

    Ok(())
}
