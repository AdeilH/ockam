use clap::Args;
use console::Term;
use indoc::formatdoc;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::relay::RelayInfo;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use ockam_core::AsyncTryClone;
use serde::Serialize;

use crate::output::Output;
use crate::relay::util::relay_name_parser;
use crate::terminal::tui::ShowCommandTui;
use crate::terminal::PluralTerm;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts, Terminal, TerminalStream};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a Relay given its name
#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name assigned to the Relay, prefixed with 'forward_to_'. Example: 'forward_to_myrelay'
    #[arg(value_parser = relay_name_parser)]
    relay_name: Option<String>,

    /// Node which the relay belongs to
    #[arg(long, value_name = "NODE", value_parser = extract_address_value)]
    pub at: Option<String>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }
    pub fn name(&self) -> String {
        "show relay".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        ShowTui::run(
            ctx.async_try_clone().await.into_diagnostic()?,
            opts,
            self.clone(),
        )
        .await
    }
}

pub struct ShowTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node: BackgroundNodeClient,
    cmd: ShowCommand,
}

impl ShowTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        cmd: ShowCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(&ctx, &opts.state, &cmd.at).await?;
        let tui = Self {
            ctx,
            opts,
            node,
            cmd,
        };
        tui.show().await
    }
}

#[ockam_core::async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Relay;

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.cmd.relay_name.as_deref()
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        self.cmd
            .relay_name
            .clone()
            .ok_or(miette!("No relay name provided"))
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        let relays: Vec<RelayInfo> = self
            .node
            .ask(&self.ctx, Request::get("/node/forwarder"))
            .await?;
        let names = relays
            .into_iter()
            .map(|i| i.remote_address().to_string())
            .collect();
        Ok(names)
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let relay: RelayInfo = self
            .node
            .ask(
                &self.ctx,
                Request::get(format!("/node/forwarder/{item_name}")),
            )
            .await?;
        let relay = RelayShowOutput::from(relay);
        self.terminal()
            .stdout()
            .plain(relay.output()?)
            .machine(item_name)
            .json(serde_json::to_string(&relay).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}

#[derive(Serialize)]
struct RelayShowOutput {
    pub relay_route: String,
    pub remote_address: MultiAddr,
    pub worker_address: MultiAddr,
}

impl From<RelayInfo> for RelayShowOutput {
    fn from(r: RelayInfo) -> Self {
        Self {
            relay_route: r.forwarding_route().to_string(),
            remote_address: r.remote_address_ma().into_diagnostic().unwrap(),
            worker_address: r.worker_address_ma().into_diagnostic().unwrap(),
        }
    }
}

impl Output for RelayShowOutput {
    fn output(&self) -> crate::error::Result<String> {
        Ok(formatdoc!(
            r#"
        Relay:
            Relay Route: {route}
            Remote Address: {remote_addr}
            Worker Address: {worker_addr}
        "#,
            route = self.relay_route,
            remote_addr = self.remote_address,
            worker_addr = self.worker_address,
        ))
    }

    fn list_output(&self) -> crate::error::Result<String> {
        Ok(formatdoc!(
            r#"
            Relay Route: {route}
            Remote Address: {remote_addr}
            Worker Address: {worker_addr}"#,
            route = self.relay_route,
            remote_addr = self.remote_address,
            worker_addr = self.worker_address,
        ))
    }
}
