# Deployboard on macOS

The **Deployboard macOS installer** GitHub Actions workflow runs on pushes that
change `crates/deployboard/src/**`, its build/package files, shared `common` code,
or workspace dependencies. You can also start it from **Actions → Deployboard
macOS installer → Run workflow**. For the manual button to appear, the workflow
must first be committed to the repository's default branch.

## Download and install

Each successful run publishes a new GitHub Release tagged
`deployboard-<run-number>-<run-attempt>`, containing the Apple Silicon installer.

1. Open the repository's **Releases** page and select the Deployboard build.
2. Under **Assets**, download `deployboard-macos-apple-silicon.dmg` for your M-series Mac.
3. Open the `.dmg` file.
4. Drag `deployboard.app` onto the **Applications** shortcut, then eject the disk image.
5. Open Deployboard from Applications. Requires macOS 13 or later.

These builds are ad-hoc signed, not signed with an Apple Developer ID or notarized.
If macOS blocks the first launch, use **System Settings → Privacy & Security →
Open Anyway** after trying to open the app. Only approve a build you trust. See [Apple’s first-launch instructions](https://support.apple.com/en-ca/guide/mac-help/mh40616/mac).
Apple Developer signing and notarization credentials would be needed for a
verified-developer installation experience.

## Build locally

On macOS with Xcode command line tools, Rust, and Python 3.11 or later installed:

```sh
rustup target add aarch64-apple-darwin
MACOSX_DEPLOYMENT_TARGET=13.0 cargo build --locked --release -p deployboard --bin deployboard --target aarch64-apple-darwin
bash crates/deployboard/package-macos.sh aarch64-apple-darwin apple-silicon
```

The installer is written to `dist/deployboard-macos-apple-silicon.dmg`.
