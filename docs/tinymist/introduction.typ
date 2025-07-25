
// dist/tinymist/rs
#import "mod.typ": *

#let title = if is-md-target {
  "Tinymist"
} else {
  "Introduction"
}

#show: book-page.with(title: title)

Tinymist [ˈtaɪni mɪst] is an integrated language service for #link("https://typst.app/")[Typst] [taɪpst]. You can also call it #ruby[微][wēi][霭][ǎi] in Chinese.

It contains:
- an analyzing library for Typst, see #github-link("/crates/tinymist-query/")[tinymist-query].
- a CLI for Typst, see #github-link("/crates/tinymist/")[tinymist].
  - which provides a language server for Typst, see #cross-link("/feature/language.typ")[Language Features].
  - which provides a preview server for Typst, see #cross-link("/feature/preview.typ")[Preview Feature].
- a VSCode extension for Typst, see #github-link("/editors/vscode/")[Tinymist VSCode Extension].

= Features

#include "feature/language-content.typ"

= Versioning and Release Cycle

#include "versioning.typ"

The release cycle is as follows:
- If there is a typst version update, a new major or minor version will be released intermediately. This means tinymist will always align the minor version with typst.
- If there is at least a bug or feature added this week, a new patch version will be released.

= Installation

Follow the instructions to enable tinymist in your favorite editor.
- #cross-link("/frontend/vscode.typ")[VS Cod(e,ium)]
- #cross-link("/frontend/neovim.typ")[Neovim]
- #cross-link("/frontend/emacs.typ")[Emacs]
- #cross-link("/frontend/sublime-text.typ")[Sublime Text]
- #cross-link("/frontend/helix.typ")[Helix]
- #cross-link("/frontend/zed.typ")[Zed]

= Installing Regular/Nightly Prebuilds from GitHub

Note: if you are not knowing what is a regular/nightly release, please don't follow this section.

Besides published releases specific for each editors, you can also download the latest regular/nightly prebuilts from GitHub and install them manually.

- Regular prebuilts can be found in #link("https://github.com/Myriad-Dreamin/tinymist/releases")[GitHub Releases].
- Nightly prebuilts can be found in #link("https://github.com/Myriad-Dreamin/tinymist/actions")[GitHub Actions].
  - (Suggested) Use the #link("https://github.com/hongjr03/tinymist-nightly-installer")[tinymist-nightly-installer] to install the nightly prebuilts automatically.
    - Unix (Bash):
      ```bash
      curl -sSL https://github.com/hongjr03/tinymist-nightly-installer/releases/latest/download/run.sh | bash
      ```
    - Windows (PowerShell):
      ```bash
      iwr https://github.com/hongjr03/tinymist-nightly-installer/releases/latest/download/run.ps1 -UseBasicParsing | iex
      ```
  - The prebuilts for other revisions can also be found manually. For example, if you are seeking a nightly release for the featured #link("https://github.com/Myriad-Dreamin/tinymist/pull/468")[PR: build: bump version to 0.11.17-rc1], you could click and go to the #link("https://github.com/Myriad-Dreamin/tinymist/actions/runs/10120639466")[action page] run for the related commits and download the artifacts.

To install extension file (the file with `.vsix` extension) manually, please #kbd("Ctrl+Shift+X") in the editor window and drop the downloaded vsix file into the opened extensions view.

= Documentation

See #link("https://myriad-dreamin.github.io/tinymist/")[Online Documentation].

= Packaging

#let channel-img(lnk, img) = if sys-is-html-target {
  html.elem("a", attrs: (
    href: lnk,
    target: "_blank",
    rel: "noopener noreferrer",
  ))[
    #html.elem("img", attrs: (src: img, alt: "Packaging status", style: "max-width: 100%; height: auto;"))
  ]
} else {
  link(lnk)
}

Stable Channel:

#channel-img(
  "https://repology.org/project/tinymist/versions",
  "https://repology.org/badge/vertical-allrepos/tinymist.svg",
)

Nightly Channel:

#channel-img(
  "https://repology.org/project/tinymist-nightly/versions",
  "https://repology.org/badge/vertical-allrepos/tinymist-nightly.svg",
)

= Roadmap

== Short Terms

To encourage contributions, we create many #link("https://github.com/Myriad-Dreamin/tinymist/pulls")[Pull Requests] in draft to navigate short-term plans. They give you a hint of what or where to start in this large repository.

== Long Terms

We are planning to implement the following features in typst v0.14.0 or spare time in weekend:

- Type checking: complete the type checker.
- Periscope renderer: It is disabled since vscode reject to render SVGs containing foreignObjects.
- Inlay hint: It is disabled _by default_ because of performance issues.
- Find references of dictionary fields and named function arguments.
- Improve symbol view's appearance.
- Improve package view.
  - Navigate to symbols by clicking on the symbol name in the view.
  - Automatically locate the symbol item in the view when viewing local documentation.
  - Remember the recently invoked package commands, e.g. "Open Docs of \@preview/cetz:0.3.1", "Open directory of \@preview/touying:0.5.3".
- Improve label view.
  - Group labels.
  - Search labels.
  - Keep (persist) group preferences.
- Improve Typst Preview.
  - Pin drop-down: Set the file to preview in the drop-down for clients that doesn't support passing arguments to the preview command.
  - Render in web worker (another thread) to reduce overhead on the electron's main thread.
- #strike[Spell checking: There is already a branch but no suitable (default) spell checking library is found.]
  - #link("https://github.com/crate-ci/typos")[typos] is great for typst. #link("https://github.com/Automattic/harper")[harper] looks promise.

If you are interested by any above features, please feel free to send Issues to discuss or PRs to implement to #link("https://github.com/Myriad-Dreamin/tinymist")[GitHub.]

= Contributing

Please read the #link("CONTRIBUTING.md")[CONTRIBUTING.md] file for contribution guidelines.

= Sponsoring

Tinymist thrives on community love and remains proudly independent. While we don't accept direct project funding, we warmly welcome support for our maintainers' personal efforts. Please go to #github-link("/MAINTAINERS.md")[Maintainers Page] and #link("https://github.com/Myriad-Dreamin/tinymist/graphs/contributors")[Contributors Page] and find their personal pages for more information. It is also welcomed to directly ask questions about sponsoring on the #link("https://github.com/Myriad-Dreamin/tinymist/issues/new")[GitHub Issues].

= Acknowledgements

- Partially code is inherited from #link("https://github.com/nvarner/typst-lsp")[typst-lsp]
- The #github-link("/editors/vscode#symbol-view")[integrating] *offline* handwritten-stroke recognizer is powered by #link("https://detypify.quarticcat.com/")[Detypify].
- The #github-link("/editors/vscode#preview-command")[integrating] preview service is powered by #link("https://github.com/Enter-tainer/typst-preview")[typst-preview].
- The #github-link("/editors/vscode#managing-local-packages")[integrating] local package management functions are adopted from #link("https://github.com/OrangeX4/vscode-typst-sync")[vscode-typst-sync].
