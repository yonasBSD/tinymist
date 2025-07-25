#![allow(missing_docs)]

use std::sync::Arc;

use reflexo_typst::{Bytes, CompilerFeat, EntryReader, ExportWebSvgHtmlTask, WebSvgHtmlExport};
use reflexo_vec2svg::DefaultExportFeature;
use tinymist_std::error::prelude::*;
use tinymist_std::typst::TypstPagedDocument;
use tinymist_task::{ExportTimings, TextExport};
use typlite::{Format, Typlite};

use crate::project::{
    ExportTeXTask, HtmlExport, LspCompilerFeat, PdfExport, PngExport, ProjectTask, SvgExport,
    TaskWhen,
};
use crate::world::base::{
    ConfigTask, DiagnosticsTask, ExportComputation, FlagTask, HtmlCompilationTask,
    OptionDocumentTask, PagedCompilationTask, WorldComputable, WorldComputeGraph,
};

#[derive(Clone, Copy, Default)]
pub struct ProjectCompilation;

impl ProjectCompilation {
    pub fn preconfig_timings<F: CompilerFeat>(graph: &Arc<WorldComputeGraph<F>>) -> Result<bool> {
        // todo: configure run_diagnostics!
        let paged_diag = Some(TaskWhen::OnType);
        let paged_diag2 = Some(TaskWhen::Script);
        let html_diag = Some(TaskWhen::Never);

        let pdf: Option<TaskWhen> = graph
            .get::<ConfigTask<<PdfExport as ExportComputation<LspCompilerFeat, _>>::Config>>()
            .transpose()?
            .map(|config| config.export.when.clone());
        let svg: Option<TaskWhen> = graph
            .get::<ConfigTask<<SvgExport as ExportComputation<LspCompilerFeat, _>>::Config>>()
            .transpose()?
            .map(|config| config.export.when.clone());
        let png: Option<TaskWhen> = graph
            .get::<ConfigTask<<PngExport as ExportComputation<LspCompilerFeat, _>>::Config>>()
            .transpose()?
            .map(|config| config.export.when.clone());
        let html: Option<TaskWhen> = graph
            .get::<ConfigTask<<HtmlExport as ExportComputation<LspCompilerFeat, _>>::Config>>()
            .transpose()?
            .map(|config| config.export.when.clone());
        let md: Option<TaskWhen> = graph
            .get::<ConfigTask<ExportTeXTask>>()
            .transpose()?
            .map(|config| config.export.when.clone());
        let text: Option<TaskWhen> = graph
            .get::<ConfigTask<<TextExport as ExportComputation<LspCompilerFeat, _>>::Config>>()
            .transpose()?
            .map(|config| config.export.when.clone());

        let doc = None::<TypstPagedDocument>.as_ref();
        let check = |timing: Option<TaskWhen>| {
            ExportTimings::needs_run(&graph.snap, timing.as_ref(), doc).unwrap_or(true)
        };

        let compile_paged = [paged_diag, paged_diag2, pdf, svg, png, text, md]
            .into_iter()
            .any(check);
        let compile_html = [html_diag, html].into_iter().any(check);

        let _ = graph.provide::<FlagTask<PagedCompilationTask>>(Ok(FlagTask::flag(compile_paged)));
        let _ = graph.provide::<FlagTask<HtmlCompilationTask>>(Ok(FlagTask::flag(compile_html)));

        Ok(compile_paged || compile_html)
    }
}

impl<F: CompilerFeat> WorldComputable<F> for ProjectCompilation {
    type Output = Self;

    fn compute(graph: &Arc<WorldComputeGraph<F>>) -> Result<Self> {
        Self::preconfig_timings(graph)?;
        DiagnosticsTask::compute(graph)?;
        Ok(Self)
    }
}

pub struct ProjectExport;

impl ProjectExport {
    fn export_bytes<
        D: typst::Document + Send + Sync + 'static,
        T: ExportComputation<LspCompilerFeat, D, Output = Bytes>,
    >(
        graph: &Arc<WorldComputeGraph<LspCompilerFeat>>,
        when: Option<&TaskWhen>,
        config: &T::Config,
    ) -> Result<Option<Bytes>> {
        let doc = graph.compute::<OptionDocumentTask<D>>()?;
        let doc = doc.as_ref();
        let n = ExportTimings::needs_run(&graph.snap, when, doc.as_deref()).unwrap_or(true);
        if !n {
            return Ok(None);
        }

        let res = doc.as_ref().map(|doc| T::run(graph, doc, config));
        res.transpose()
    }

    fn export_string<
        D: typst::Document + Send + Sync + 'static,
        T: ExportComputation<LspCompilerFeat, D, Output = String>,
    >(
        graph: &Arc<WorldComputeGraph<LspCompilerFeat>>,
        when: Option<&TaskWhen>,
        config: &T::Config,
    ) -> Result<Option<Bytes>> {
        let doc = graph.compute::<OptionDocumentTask<D>>()?;
        let doc = doc.as_ref();
        let n = ExportTimings::needs_run(&graph.snap, when, doc.as_deref()).unwrap_or(true);
        if !n {
            return Ok(None);
        }

        let doc = doc.as_ref();
        let res = doc.map(|doc| T::run(graph, doc, config).map(Bytes::from_string));
        res.transpose()
    }
}

impl WorldComputable<LspCompilerFeat> for ProjectExport {
    type Output = Self;

    fn compute(graph: &Arc<WorldComputeGraph<LspCompilerFeat>>) -> Result<Self> {
        let config = graph.must_get::<ConfigTask<ProjectTask>>()?;
        let output_path = config.as_export().and_then(|e| {
            e.output
                .as_ref()
                .and_then(|o| o.substitute(&graph.snap.world.entry_state()))
        });
        let when = config.when();

        let output = || -> Result<Option<Bytes>> {
            use ProjectTask::*;
            match config.as_ref() {
                Preview(..) => todo!(),
                ExportPdf(config) => Self::export_bytes::<_, PdfExport>(graph, when, config),
                ExportPng(config) => Self::export_bytes::<_, PngExport>(graph, when, config),
                ExportSvg(config) => Self::export_string::<_, SvgExport>(graph, when, config),
                ExportHtml(config) => Self::export_string::<_, HtmlExport>(graph, when, config),
                // todo: configuration
                ExportSvgHtml(_config) => Self::export_string::<
                    _,
                    WebSvgHtmlExport<DefaultExportFeature>,
                >(
                    graph, when, &ExportWebSvgHtmlTask::default()
                ),
                ExportMd(..) => {
                    let doc = graph.compute::<OptionDocumentTask<TypstPagedDocument>>()?;
                    let doc = doc.as_ref();
                    let n =
                        ExportTimings::needs_run(&graph.snap, when, doc.as_deref()).unwrap_or(true);
                    if !n {
                        return Ok(None);
                    }

                    Ok(TypliteMdExport::run(graph)?.map(Bytes::from_string))
                }
                ExportTeX(..) => {
                    let doc = graph.compute::<OptionDocumentTask<TypstPagedDocument>>()?;
                    let doc = doc.as_ref();
                    let n =
                        ExportTimings::needs_run(&graph.snap, when, doc.as_deref()).unwrap_or(true);
                    if !n {
                        return Ok(None);
                    }

                    Ok(TypliteTeXExport::run(graph)?.map(Bytes::from_string))
                }
                ExportText(config) => Self::export_string::<_, TextExport>(graph, when, config),
                Query(..) => todo!(),
            }
        };

        if let Some(path) = output_path {
            let output = output()?;
            // todo: don't ignore export source diagnostics
            if let Some(output) = output {
                std::fs::write(path, output).context("failed to write output")?;
            }
        }

        Ok(Self {})
    }
}

pub struct TypliteExport<const FORMAT: char>;

const fn typlite_format(f: char) -> Format {
    match f {
        'm' => Format::Md,
        'x' => Format::LaTeX,
        _ => panic!("unsupported format for TypliteExport"),
    }
}

const fn typlite_name(f: char) -> &'static str {
    match f {
        'm' => "Markdown",
        'x' => "LaTeX",
        _ => panic!("unsupported format for TypliteExport"),
    }
}

impl<const F: char> TypliteExport<F> {
    fn run(graph: &Arc<WorldComputeGraph<LspCompilerFeat>>) -> Result<Option<String>> {
        let conv = Typlite::new(Arc::new(graph.snap.world.clone()))
            .with_format(typlite_format(F))
            .convert()
            .map_err(|e| anyhow::anyhow!("failed to convert to {}: {e}", typlite_name(F)))?;

        Ok(Some(conv.to_string()))
    }
}

impl<const F: char> WorldComputable<LspCompilerFeat> for TypliteExport<F> {
    type Output = Option<String>;

    fn compute(graph: &Arc<WorldComputeGraph<LspCompilerFeat>>) -> Result<Self::Output> {
        Self::run(graph)
    }
}

pub type TypliteMdExport = TypliteExport<'m'>;
pub type TypliteTeXExport = TypliteExport<'x'>;
