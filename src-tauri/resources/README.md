# Bundle resources

ProxyBot's optional APK patching feature needs Apktool 3.0.2 and Frida Gadget 17.12.0 for the Android architectures supported by Frida.

The binaries are intentionally not committed to Git. Fetch the pinned, checksum-verified copies before creating a Tauri bundle:

```bash
pnpm resources:fetch
```

`pnpm build:tauri` runs this automatically through `tauri.bundle.conf.json`. Ordinary Cargo tests and checks use the base Tauri configuration and do not require these optional release assets. For an offline bundle, populate the paths listed in [`resources.lock`](resources.lock) ahead of time; `pnpm resources:check` verifies the files without making network requests.

The downloaded files remain third-party works under their upstream licenses. See the [Apktool repository](https://github.com/iBotPeaches/Apktool) and [Frida repository](https://github.com/frida/frida) for their source and license terms.
