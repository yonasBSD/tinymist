Tinymist's versions follow the #link("https://semver.org/")[Semantic Versioning] scheme, in format of `MAJOR.MINOR.PATCH`. Besides, tinymist follows special rules for the version number:
- If a version is suffixed with `-rcN` ($N > 0$), e.g. `0.11.0-rc1` and `0.12.1-rc1`, it means this version is a release candidate. It is used to test publish script and E2E functionalities. These versions will not be published to the marketplace.
- If the `PATCH` number is odd, e.g. `0.11.1` and `0.12.3`, it means this version is a nightly release. The nightly release will use both #link("https://github.com/Myriad-Dreamin/tinymist/tree/main")[tinymist] and #link("https://github.com/typst/typst/tree/main")[typst] at *main branch*. They will be published as prerelease version to the marketplace. Note that in nightly releases, we change `#sys.version` to the next minor release to help develop documents with nightly features. For example, in tinymist nightly v0.12.1 or v0.12.3, the `#sys.version` is changed to `version(0, 13, 0)`.
- Otherwise, if the `PATCH` number is even, e.g. `0.11.0` and `0.12.2`, it means this version is a regular release. The regular release will always use the recent stable version of tinymist and typst.