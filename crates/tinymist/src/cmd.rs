//! Tinymist LSP commands

use std::ops::{Deref, Range};
use std::path::PathBuf;

use lsp_types::TextDocumentIdentifier;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sync_ls::RequestId;
use task::TraceParams;
use tinymist_assets::TYPST_PREVIEW_HTML;
use tinymist_project::{
    ExportHtmlTask, ExportPdfTask, ExportPngTask, ExportSvgTask, ExportTask, ExportTeXTask,
    ExportTextTask, ExportTransform, PageSelection, Pages, ProjectTask, QueryTask,
};
use tinymist_query::package::PackageInfo;
use tinymist_query::{LocalContextGuard, LspRange};
use tinymist_std::error::prelude::*;
use tinymist_task::ExportMarkdownTask;
use typst::diag::{eco_format, EcoString, StrResult};
use typst::syntax::package::{PackageSpec, VersionlessPackageSpec};
use typst::syntax::{LinkedNode, Source};
use world::TaskInputs;

use super::*;
use crate::lsp::query::{run_query, LspClientExt};
use crate::tool::ast::AstRepr;
use crate::tool::package::InitTask;

/// See [`ProjectTask`].
#[derive(Debug, Clone, Default, Deserialize)]
struct ExportOpts {
    fill: Option<String>,
    ppi: Option<f32>,
    #[serde(default)]
    page: PageSelection,
    /// Whether to open the exported file(s) after the export is done.
    open: Option<bool>,
    // todo: we made a mistake that they will be snakecase, but they should be camelCase
    /// The creation timestamp for various outputs (in seconds).
    creation_timestamp: Option<String>,
    /// A PDF standard that Typst can enforce conformance with.
    pdf_standard: Option<Vec<PdfStandard>>,
}

/// See [`ProjectTask`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportTypliteOpts {
    /// Whether to open the exported file(s) after the export is done.
    open: Option<bool>,
    /// The processor to use for the typlite export.
    processor: Option<String>,
    /// The path of external assets directory.
    assets_path: Option<PathBuf>,
}

/// See [`ProjectTask`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueryOpts {
    format: String,
    output_extension: Option<String>,
    strict: Option<bool>,
    pretty: Option<bool>,
    selector: String,
    field: Option<String>,
    one: Option<bool>,
    /// Whether to open the exported file(s) after the export is done.
    open: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportSyntaxRangeOpts {
    range: Option<LspRange>,
}

/// Here are implemented the handlers for each command.
impl ServerState {
    /// Export the current document as PDF file(s).
    pub fn export_pdf(&mut self, req_id: RequestId, mut args: Vec<JsonValue>) -> ScheduledResult {
        let opts = get_arg_or_default!(args[1] as ExportOpts);

        let creation_timestamp = if let Some(value) = opts.creation_timestamp {
            Some(
                parse_source_date_epoch(&value)
                    .map_err(|e| invalid_params(format!("Cannot parse creation timestamp: {e}")))?,
            )
        } else {
            self.config.creation_timestamp()
        };
        let pdf_standards = opts.pdf_standard.or_else(|| self.config.pdf_standards());

        let export = self.config.export_task();
        self.export(
            req_id,
            ProjectTask::ExportPdf(ExportPdfTask {
                export,
                pdf_standards: pdf_standards.unwrap_or_default(),
                creation_timestamp,
            }),
            opts.open.unwrap_or_default(),
            args,
        )
    }

    /// Export the current document as HTML file(s).
    pub fn export_html(&mut self, req_id: RequestId, mut args: Vec<JsonValue>) -> ScheduledResult {
        let opts = get_arg_or_default!(args[1] as ExportOpts);
        let export = self.config.export_task();
        self.export(
            req_id,
            ProjectTask::ExportHtml(ExportHtmlTask { export }),
            opts.open.unwrap_or_default(),
            args,
        )
    }

    /// Export the current document as Markdown file(s).
    pub fn export_markdown(
        &mut self,
        req_id: RequestId,
        mut args: Vec<JsonValue>,
    ) -> ScheduledResult {
        let opts = get_arg_or_default!(args[1] as ExportTypliteOpts);
        let export = self.config.export_task();
        self.export(
            req_id,
            ProjectTask::ExportMd(ExportMarkdownTask {
                processor: opts.processor,
                assets_path: opts.assets_path,
                export,
            }),
            opts.open.unwrap_or_default(),
            args,
        )
    }

    /// Export the current document as Tex file(s).
    pub fn export_tex(&mut self, req_id: RequestId, mut args: Vec<JsonValue>) -> ScheduledResult {
        let opts = get_arg_or_default!(args[1] as ExportTypliteOpts);
        let export = self.config.export_task();
        self.export(
            req_id,
            ProjectTask::ExportTeX(ExportTeXTask {
                processor: opts.processor,
                assets_path: opts.assets_path,
                export,
            }),
            opts.open.unwrap_or_default(),
            args,
        )
    }

    /// Export the current document as Text file(s).
    pub fn export_text(&mut self, req_id: RequestId, mut args: Vec<JsonValue>) -> ScheduledResult {
        let opts = get_arg_or_default!(args[1] as ExportOpts);
        let export = self.config.export_task();
        self.export(
            req_id,
            ProjectTask::ExportText(ExportTextTask { export }),
            opts.open.unwrap_or_default(),
            args,
        )
    }

    /// Query the current document and export the result as JSON file(s).
    pub fn export_query(&mut self, req_id: RequestId, mut args: Vec<JsonValue>) -> ScheduledResult {
        let opts = get_arg_or_default!(args[1] as QueryOpts);
        // todo: deprecate it
        let _ = opts.strict;

        let mut export = self.config.export_task();
        if opts.pretty.unwrap_or(true) {
            export.apply_pretty();
        }

        self.export(
            req_id,
            ProjectTask::Query(QueryTask {
                format: opts.format,
                output_extension: opts.output_extension,
                selector: opts.selector,
                field: opts.field,
                one: opts.one.unwrap_or(false),
                export,
            }),
            opts.open.unwrap_or_default(),
            args,
        )
    }

    /// Export the current document as Svg file(s).
    pub fn export_svg(&mut self, req_id: RequestId, mut args: Vec<JsonValue>) -> ScheduledResult {
        let opts = get_arg_or_default!(args[1] as ExportOpts);

        let mut export = self.config.export_task();
        select_page(&mut export, opts.page).map_err(invalid_params)?;

        self.export(
            req_id,
            ProjectTask::ExportSvg(ExportSvgTask { export }),
            opts.open.unwrap_or_default(),
            args,
        )
    }

    /// Export the current document as Png file(s).
    pub fn export_png(&mut self, req_id: RequestId, mut args: Vec<JsonValue>) -> ScheduledResult {
        let opts = get_arg_or_default!(args[1] as ExportOpts);

        let ppi = opts.ppi.unwrap_or(144.);
        let ppi = ppi
            .try_into()
            .context("cannot convert ppi")
            .map_err(invalid_params)?;

        let mut export = self.config.export_task();
        select_page(&mut export, opts.page).map_err(invalid_params)?;

        self.export(
            req_id,
            ProjectTask::ExportPng(ExportPngTask {
                fill: opts.fill,
                ppi,
                export,
            }),
            opts.open.unwrap_or_default(),
            args,
        )
    }

    /// Export the current document as some format. The client is responsible
    /// for passing the correct absolute path of typst document.
    pub fn export(
        &mut self,
        req_id: RequestId,
        task: ProjectTask,
        open: bool,
        mut args: Vec<JsonValue>,
    ) -> ScheduledResult {
        let path = get_arg!(args[0] as PathBuf);

        run_query!(req_id, self.OnExport(path, open, task))
    }

    /// Export a range of the current document as Ansi highlighted text.
    pub fn export_ansi_hl(&mut self, mut args: Vec<JsonValue>) -> AnySchedulableResponse {
        let path = get_arg!(args[0] as PathBuf);
        let opts = get_arg_or_default!(args[1] as ExportSyntaxRangeOpts);

        let output = self.select_range(path, opts.range, |source, range| {
            let mut text_in_range = source.text();
            if let Some(range) = range {
                text_in_range = text_in_range
                    .get(range)
                    .ok_or_else(|| internal_error("cannot get text in range"))?;
            }

            typst_ansi_hl::Highlighter::default()
                .for_discord()
                .with_soft_limit(2000)
                .highlight(text_in_range)
                .map_err(|e| internal_error(format!("cannot highlight: {e}")))
        })?;

        just_ok(JsonValue::String(output))
    }

    /// Export a range of the current file's AST.
    pub fn export_ast(&mut self, mut args: Vec<JsonValue>) -> AnySchedulableResponse {
        let path = get_arg!(args[0] as PathBuf);
        let opts = get_arg_or_default!(args[1] as ExportSyntaxRangeOpts);

        let output = self.select_range(path, opts.range, |source, range| {
            let linked_node = LinkedNode::new(source.root());
            Ok(format!("{}", AstRepr(linked_node, range)))
        })?;

        just_ok(JsonValue::String(output))
    }

    fn select_range<T>(
        &mut self,
        path: PathBuf,
        range: Option<LspRange>,
        f: impl Fn(Source, Option<Range<usize>>) -> LspResult<T>,
    ) -> LspResult<T> {
        let s = self
            .query_source(path.into(), Ok)
            .map_err(|e| internal_error(format!("cannot find source: {e}")))?;

        // todo: cannot select syntax-sensitive data well
        // let node = LinkedNode::new(s.root());

        let range = range
            .map(|r| {
                tinymist_query::to_typst_range(r, self.const_config().position_encoding, &s)
                    .ok_or_else(|| internal_error("cannoet convert range"))
            })
            .transpose()?;

        f(s, range)
    }

    /// Clear all cached resources.
    pub fn clear_cache(&mut self, _arguments: Vec<JsonValue>) -> AnySchedulableResponse {
        comemo::evict(0);
        self.project.analysis.clear_cache();
        just_ok(JsonValue::Null)
    }

    /// Pin main file to some path.
    pub fn pin_document(&mut self, mut args: Vec<JsonValue>) -> AnySchedulableResponse {
        let entry = get_arg!(args[0] as Option<PathBuf>).map(From::from);

        let update_result = self.pin_main_file(entry.clone());
        update_result.map_err(|err| internal_error(format!("could not pin file: {err}")))?;

        log::info!("file pinned: {entry:?}");
        just_ok(JsonValue::Null)
    }

    /// Focus main file to some path.
    pub fn focus_document(&mut self, mut args: Vec<JsonValue>) -> AnySchedulableResponse {
        let entry = get_arg!(args[0] as Option<PathBuf>).map(From::from);

        if !self.ever_manual_focusing {
            self.ever_manual_focusing = true;
            log::info!("first manual focusing is coming");
        }

        let ok = self.focus_main_file(entry.clone());
        let ok = ok.map_err(|err| internal_error(format!("could not focus file: {err}")))?;

        if ok {
            log::info!("file focused: {entry:?}");
        }
        just_ok(JsonValue::Null)
    }

    /// Starts a preview instance.
    #[cfg(feature = "preview")]
    pub fn do_start_preview(
        &mut self,
        mut args: Vec<JsonValue>,
    ) -> SchedulableResponse<crate::tool::preview::StartPreviewResponse> {
        let cli_args = get_arg_or_default!(args[0] as Vec<String>);
        self.start_preview(cli_args, crate::tool::preview::PreviewKind::Regular)
    }

    /// Starts a preview instance for browsing.
    #[cfg(feature = "preview")]
    pub fn browse_preview(
        &mut self,
        mut args: Vec<JsonValue>,
    ) -> SchedulableResponse<crate::tool::preview::StartPreviewResponse> {
        let cli_args = get_arg_or_default!(args[0] as Vec<String>);
        self.start_preview(cli_args, crate::tool::preview::PreviewKind::Browsing)
    }

    /// Starts a preview instance but without arguments. This is used for the
    /// case where a client cannot pass arguments to the preview command. It
    /// is also an example of how to use the `preview` command.
    ///
    /// Behaviors:
    /// - The preview server listens on a random port.
    /// - The colors are inverted according to the user's system settings.
    /// - The preview follows an inferred focused file from the requests from
    ///   the client.
    /// - The preview is opened in the default browser.
    #[cfg(feature = "preview")]
    pub fn default_preview(
        &mut self,
        mut _args: Vec<JsonValue>,
    ) -> SchedulableResponse<crate::tool::preview::StartPreviewResponse> {
        let cli_args = self.config.preview.browsing.args.clone();
        let cli_args = cli_args.unwrap_or_else(|| {
            let default_args = [
                "--data-plane-host=127.0.0.1:0",
                "--control-plane-host=127.0.0.1:0",
                "--invert-colors=auto",
                "--open",
            ];
            default_args.map(ToString::to_string).to_vec()
        });
        self.start_preview(cli_args, crate::tool::preview::PreviewKind::Browsing)
    }

    /// Kill a preview instance.
    #[cfg(feature = "preview")]
    pub fn kill_preview(&mut self, mut args: Vec<JsonValue>) -> AnySchedulableResponse {
        let task_id = get_arg!(args[0] as String);

        if args.is_empty() {
            return self.preview.kill_all();
        }

        self.preview.kill(task_id)
    }

    /// Scroll preview instances.
    #[cfg(feature = "preview")]
    pub fn scroll_preview(&mut self, mut args: Vec<JsonValue>) -> AnySchedulableResponse {
        use tinymist_preview::ControlPlaneMessage;

        if args.is_empty() {
            return self.preview.scroll_all(self.infer_pos()?);
        }

        let task_id = get_arg!(args[0] as String);
        let req = get_arg!(args[1] as ControlPlaneMessage);

        self.preview.scroll(task_id, req)
    }

    /// Initialize a new template.
    pub fn init_template(&mut self, mut args: Vec<JsonValue>) -> AnySchedulableResponse {
        use crate::tool::package::{self, TemplateSource};

        #[derive(Debug, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct InitResult {
            entry_path: PathBuf,
        }

        let from_source = get_arg!(args[0] as String);
        let to_path = get_arg!(args[1] as Option<PathBuf>).map(From::from);

        let snap = self.snapshot().map_err(internal_error)?;

        just_future(async move {
            // Parse the package specification. If the user didn't specify the version,
            // we try to figure it out automatically by downloading the package index
            // or searching the disk.
            let spec: PackageSpec = from_source
                .parse()
                .or_else(|err| {
                    // Try to parse without version, but prefer the error message of the
                    // normal package spec parsing if it fails.
                    let spec: VersionlessPackageSpec = from_source.parse().map_err(|_| err)?;
                    let version = snap.registry().determine_latest_version(&spec)?;
                    StrResult::Ok(spec.at(version))
                })
                .map_err(map_string_err("failed to parse package spec"))
                .map_err(internal_error)?;

            let from_source = TemplateSource::Package(spec);

            let entry_path = package::init(
                snap.world(),
                InitTask {
                    tmpl: from_source.clone(),
                    dir: to_path.clone(),
                },
            )
            .map_err(map_string_err("failed to initialize template"))
            .map_err(internal_error)?;

            log::info!("template initialized: {from_source:?} to {to_path:?}");

            serde_json::to_value(InitResult { entry_path })
                .map_err(|_| internal_error("Cannot serialize path"))
        })
    }

    /// Get the entry of a template.
    pub fn get_template_entry(&mut self, mut args: Vec<JsonValue>) -> AnySchedulableResponse {
        use crate::tool::package::{self, TemplateSource};

        let from_source = get_arg!(args[0] as String);

        let snap = self.snapshot().map_err(internal_error)?;

        just_future(async move {
            // Parse the package specification. If the user didn't specify the version,
            // we try to figure it out automatically by downloading the package index
            // or searching the disk.
            let spec: PackageSpec = from_source
                .parse()
                .or_else(|err| {
                    // Try to parse without version, but prefer the error message of the
                    // normal package spec parsing if it fails.
                    let spec: VersionlessPackageSpec = from_source.parse().map_err(|_| err)?;
                    let version = snap.registry().determine_latest_version(&spec)?;
                    StrResult::Ok(spec.at(version))
                })
                .map_err(map_string_err("failed to parse package spec"))
                .map_err(internal_error)?;

            let from_source = TemplateSource::Package(spec);

            let entry = package::get_entry(snap.world(), from_source)
                .map_err(map_string_err("failed to get template entry"))
                .map_err(internal_error)?;

            let entry = String::from_utf8(entry.to_vec())
                .map_err(|_| invalid_params("template entry is not a valid UTF-8 string"))?;

            Ok(JsonValue::String(entry))
        })
    }

    /// Interact with the code context at the source file.
    pub fn interact_code_context(
        &mut self,
        req_id: RequestId,
        _arguments: Vec<JsonValue>,
    ) -> ScheduledResult {
        let queries = _arguments.into_iter().next().ok_or_else(|| {
            invalid_params("The first parameter is not a valid code context query array")
        })?;

        #[derive(Debug, Clone, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct InteractCodeContextParams {
            pub text_document: TextDocumentIdentifier,
            pub query: Vec<Option<tinymist_query::InteractCodeContextQuery>>,
        }

        let params: InteractCodeContextParams = serde_json::from_value(queries)
            .map_err(|e| invalid_params(format!("Cannot parse code context queries: {e}")))?;
        let path = as_path(params.text_document);
        let query = params.query;

        run_query!(req_id, self.InteractCodeContext(path, query))
    }

    /// Get the trace data of the document.
    pub fn get_document_trace(&mut self, mut args: Vec<JsonValue>) -> AnySchedulableResponse {
        let path = get_arg!(args[0] as PathBuf).into();

        // get path to self program
        let self_path = std::env::current_exe()
            .map_err(|e| internal_error(format!("Cannot get typst compiler {e}")))?;

        let entry = self.entry_resolver().resolve(Some(path));

        let snap = self.snapshot().map_err(internal_error)?;
        let user_action = self.user_action;

        just_future(async move {
            let display_entry = || format!("{entry:?}");

            // todo: rootless file
            // todo: memory dirty file
            let root = entry
                .root()
                .ok_or_else(|| error_once!("root must be determined for trace, got", entry: display_entry()))
                .map_err(internal_error)?;
            let main = entry
                .main()
                .and_then(|e| e.vpath().resolve(&root))
                .ok_or_else(
                    || error_once!("main file must be resolved, got", entry: display_entry()),
                )
                .map_err(internal_error)?;

            let task = user_action.trace_document(TraceParams {
                compiler_program: self_path,
                root: root.as_ref().to_owned(),
                main,
                inputs: snap.world().inputs().as_ref().deref().clone(),
                font_paths: snap.world().font_resolver.font_paths().to_owned(),
                rpc_kind: "http".into(),
            })?;

            tokio::pin!(task);
            task.as_mut().await;
            task.take_output().unwrap()
        })
    }

    /// Start to get the trace data of the server.
    pub fn start_server_trace(&mut self, _args: Vec<JsonValue>) -> AnySchedulableResponse {
        let task_cell = &mut self.server_trace;
        if task_cell
            .as_ref()
            .is_some_and(|task| task.stop_tx.is_closed())
        {
            *task_cell = None;
        }

        if task_cell.is_some() {
            return Err(internal_error("server trace is already started"));
        }

        let (task, resp) = self.user_action.trace_server();
        *task_cell = Some(task);

        log::info!("server trace started");

        resp
    }

    /// Stop getting the trace data of the server.
    pub fn stop_server_trace(&mut self, _args: Vec<JsonValue>) -> AnySchedulableResponse {
        let task_cell = &mut self.server_trace;
        if task_cell
            .as_ref()
            .is_some_and(|task| task.stop_tx.is_closed())
        {
            log::info!("server trace is dropped");
            *task_cell = None;
        }

        let Some(task) = task_cell.take() else {
            return Err(internal_error("server trace is not started or stopped"));
        };

        if task.stop_tx.send(()).is_err() {
            return Err(internal_error("cannot send stop signal to server trace"));
        }

        log::info!("server trace stopping");
        just_future(async move { task.resp_rx.await.map_err(internal_error)? })
    }

    /// Get the metrics of the document.
    pub fn get_document_metrics(
        &mut self,
        req_id: RequestId,
        mut args: Vec<JsonValue>,
    ) -> ScheduledResult {
        let path = get_arg!(args[0] as PathBuf);
        run_query!(req_id, self.DocumentMetrics(path))
    }

    /// Get all syntactic labels in workspace.
    pub fn get_workspace_labels(
        &mut self,
        req_id: RequestId,
        _arguments: Vec<JsonValue>,
    ) -> ScheduledResult {
        run_query!(req_id, self.WorkspaceLabel())
    }

    /// Get the server info.
    pub fn get_server_info(
        &mut self,
        req_id: RequestId,
        _arguments: Vec<JsonValue>,
    ) -> ScheduledResult {
        run_query!(req_id, self.ServerInfo())
    }
}

impl ServerState {
    /// Get the all valid fonts
    pub fn resource_fonts(&mut self, _arguments: Vec<JsonValue>) -> AnySchedulableResponse {
        let snapshot = self.snapshot().map_err(internal_error)?;
        just_future(Self::get_font_resources(snapshot))
    }

    /// Get the all valid symbols
    pub fn resource_symbols(&mut self, _arguments: Vec<JsonValue>) -> AnySchedulableResponse {
        let snapshot = self.snapshot().map_err(internal_error)?;
        just_future(Self::get_symbol_resources(snapshot))
    }

    /// Get resource preview html
    pub fn resource_preview_html(&mut self, _arguments: Vec<JsonValue>) -> AnySchedulableResponse {
        let resp = serde_json::to_value(TYPST_PREVIEW_HTML);
        just_result(resp.map_err(|e| internal_error(e.to_string())))
    }

    /// Get tutorial web page
    pub fn resource_tutoral(&mut self, _arguments: Vec<JsonValue>) -> AnySchedulableResponse {
        Err(method_not_found())
    }

    /// Get directory of packages
    pub fn resource_package_dirs(&mut self, _arguments: Vec<JsonValue>) -> AnySchedulableResponse {
        let snap = self.snapshot().map_err(internal_error)?;
        just_future(async move {
            let paths = snap.registry().paths();
            let paths = paths.iter().map(|p| p.as_ref()).collect::<Vec<_>>();
            serde_json::to_value(paths).map_err(|e| internal_error(e.to_string()))
        })
    }

    /// Get writable directory of packages
    pub fn resource_local_package_dir(
        &mut self,
        _arguments: Vec<JsonValue>,
    ) -> AnySchedulableResponse {
        let snap = self.snapshot().map_err(internal_error)?;
        just_future(async move {
            let paths = snap.registry().local_path();
            let paths = paths.as_deref().into_iter().collect::<Vec<_>>();
            serde_json::to_value(paths).map_err(|e| internal_error(e.to_string()))
        })
    }

    /// Get writable directory of packages
    pub fn resource_package_by_ns(
        &mut self,
        mut arguments: Vec<JsonValue>,
    ) -> AnySchedulableResponse {
        let ns = get_arg!(arguments[1] as EcoString);

        let snap = self.snapshot().map_err(internal_error)?;
        just_future(async move {
            let packages = tinymist_query::package::list_package_by_namespace(snap.registry(), ns)
                .into_iter()
                .map(PackageInfo::from)
                .collect::<Vec<_>>();

            serde_json::to_value(packages).map_err(|e| internal_error(e.to_string()))
        })
    }

    /// Get the all valid symbols
    pub fn resource_package_symbols(
        &mut self,
        mut arguments: Vec<JsonValue>,
    ) -> AnySchedulableResponse {
        let snap = self.query_snapshot().map_err(internal_error)?;
        let info = get_arg!(arguments[1] as PackageInfo);

        just_future(async move {
            let symbols = snap
                .run_analysis(|a| {
                    tinymist_query::docs::package_module_docs(a, &info)
                        .map_err(map_string_err("failed to list symbols"))
                })
                .map_err(internal_error)?
                .map_err(internal_error)?;

            serde_json::to_value(symbols).map_err(internal_error)
        })
    }

    // todo: it looks like we can generate this function
    /// Get the all symbol docs
    pub fn resource_package_docs(
        &mut self,
        mut arguments: Vec<JsonValue>,
    ) -> AnySchedulableResponse {
        let info = get_arg!(arguments[1] as PackageInfo);

        let fut = self.resource_package_docs_(info)?;
        just_future(async move { serde_json::to_value(fut.await?).map_err(internal_error) })
    }

    /// Get the all symbol docs
    pub fn resource_package_docs_(
        &mut self,
        info: PackageInfo,
    ) -> LspResult<impl Future<Output = LspResult<String>>> {
        self.within_package(info.clone(), move |a| {
            tinymist_query::docs::package_docs(a, &info)
                .map_err(map_string_err("failed to generate docs"))
                .map_err(internal_error)
        })
    }

    /// Check package
    pub fn check_package(
        &mut self,
        info: PackageInfo,
    ) -> LspResult<impl Future<Output = LspResult<()>>> {
        self.within_package(info.clone(), move |a| {
            tinymist_query::package::check_package(a, &info)
                .map_err(map_string_err("failed to check package"))
                .map_err(internal_error)
        })
    }

    /// Check within package
    pub fn within_package<T>(
        &mut self,
        info: PackageInfo,
        f: impl FnOnce(&mut LocalContextGuard) -> LspResult<T> + Send + Sync,
    ) -> LspResult<impl Future<Output = LspResult<T>>> {
        let snap = self.query_snapshot().map_err(internal_error)?;

        Ok(async move {
            let world = snap.world();

            let entry: StrResult<EntryState> = Ok(()).and_then(|_| {
                let toml_id = tinymist_query::package::get_manifest_id(&info)?;
                let toml_path = world.path_for_id(toml_id)?.as_path().to_owned();
                let pkg_root = toml_path.parent().ok_or_else(|| {
                    eco_format!("cannot get package root (parent of {toml_path:?})")
                })?;

                let manifest = tinymist_query::package::get_manifest(world, toml_id)?;
                let entry_point = toml_id.join(&manifest.package.entrypoint);

                Ok(EntryState::new_rooted_by_id(pkg_root.into(), entry_point))
            });
            let entry = entry.map_err(|e| internal_error(e.to_string()))?;

            let snap = snap.task(TaskInputs {
                entry: Some(entry),
                inputs: None,
            });

            snap.run_analysis(f).map_err(internal_error)?
        })
    }
}

/// Applies page selection to the export task.
fn select_page(task: &mut ExportTask, selection: PageSelection) -> Result<()> {
    match selection {
        PageSelection::First => task.transform.push(ExportTransform::Pages {
            ranges: vec![Pages::FIRST],
        }),
        PageSelection::Merged { gap } => {
            task.transform.push(ExportTransform::Merge { gap });
        }
    }

    Ok(())
}
