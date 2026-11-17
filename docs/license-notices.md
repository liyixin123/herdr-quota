# 许可证与第三方声明

## 项目许可证

herdr-quota（含源码仓库全部内容）以 MIT 许可证发布，全文见仓库根目录的 [LICENSE](../LICENSE)。

许可证选择记录：维护者于 2026-11-17 在项目会话中确认采用 MIT。`Cargo.toml` 的
`license = "MIT"`、根目录 LICENSE 与本文件保持一致。

## 源码分发

从本仓库获取的源码（`src/`、`tests/`、`scripts/`、`docs/`、`examples/`、构建文件及
本文件）整体按 MIT 提供；下游复制、修改和再分发时随附 LICENSE 中的版权与许可声明
即可。

## 预编译发行包分发

GitHub Release 提供的 `herdr-quota` 预编译二进制（见[预编译发行包](releases.md)）
同样按 MIT 分发。二进制在构建时静态嵌入了下表列出的第三方 Rust crate；本项目不以
源码形式修改这些组件，它们按各自上游公布的许可条款提供，各版本的完整许可证文本与
版权声明见其上游仓库及 crates.io 对应版本。macOS 构建还静态链接 Rust 标准库及其
随附库（MIT OR Apache-2.0，见 [rust-lang/rust](https://github.com/rust-lang/rust)）。
发行包与源码同在 `liyixin123/herdr-quota` 发布，第三方条款的获取途径对任何拿到
发行包的用户同样可用。

## 第三方 Rust 依赖

以下为 0.1.3 的 `Cargo.lock` 完整锁定依赖集（包括构建期和目标平台相关依赖）；并非每个组件都会链接到每个平台的发行二进制。直接依赖为 serde、serde_json、sha2、toml_edit、ureq 和 base64。HTTP 使用 ureq，TLS 使用 rustls 与 ring，证书验证使用系统信任库（rustls-platform-verifier）。

| crate | 版本 | 许可证 | 上游仓库 |
| --- | --- | --- | --- |
| base64 | 0.22.1 | MIT OR Apache-2.0 | https://github.com/marshallpierce/rust-base64 |
| base64 | 0.23.1 | MIT OR Apache-2.0 | https://github.com/marshallpierce/rust-base64 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | https://github.com/bitflags/bitflags |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 | https://github.com/RustCrypto/utils |
| bytes | 1.12.1 | MIT | https://github.com/tokio-rs/bytes |
| cc | 1.11.2 | MIT OR Apache-2.0 | https://github.com/rust-lang/cc-rs |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | https://github.com/rust-lang/cfg-if |
| combine | 4.6.9 | MIT | https://github.com/Marwes/combine |
| core-foundation | 0.10.1 | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 | https://github.com/RustCrypto/utils |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 | https://github.com/RustCrypto/traits |
| digest | 0.10.7 | MIT OR Apache-2.0 | https://github.com/RustCrypto/traits |
| equivalent | 1.0.3 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/equivalent |
| find-msvc-tools | 0.1.15 | MIT OR Apache-2.0 | https://github.com/rust-lang/cc-rs |
| generic-array | 0.14.7 | MIT | https://github.com/fizyk20/generic-array.git |
| getrandom | 0.2.17 | MIT OR Apache-2.0 | https://github.com/rust-random/getrandom |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| http | 1.5.0 | MIT OR Apache-2.0 | https://github.com/hyperium/http |
| httparse | 1.10.1 | MIT OR Apache-2.0 | https://github.com/seanmonstar/httparse |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/indexmap |
| itoa | 1.0.18 | MIT OR Apache-2.0 | https://github.com/dtolnay/itoa |
| jni | 0.22.4 | MIT OR Apache-2.0 | https://github.com/jni-rs/jni-rs |
| jni-macros | 0.22.4 | MIT OR Apache-2.0 | https://github.com/jni-rs/jni-rs |
| jni-sys | 0.4.1 | MIT OR Apache-2.0 | https://github.com/jni-rs/jni-sys |
| jni-sys-macros | 0.4.1 | MIT OR Apache-2.0 | https://github.com/jni-rs/jni-sys |
| libc | 0.2.191 | MIT OR Apache-2.0 | https://github.com/rust-lang/libc |
| log | 0.4.35 | MIT OR Apache-2.0 | https://github.com/rust-lang/log |
| memchr | 2.8.3 | Unlicense OR MIT | https://github.com/BurntSushi/memchr |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | https://github.com/matklad/once_cell |
| openssl-probe | 0.2.2 | MIT OR Apache-2.0 | https://github.com/rustls/openssl-probe |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | https://github.com/servo/rust-url/ |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | https://github.com/dtolnay/proc-macro2 |
| quote | 1.0.47 | MIT OR Apache-2.0 | https://github.com/dtolnay/quote |
| ring | 0.17.14 | Apache-2.0 AND ISC | https://github.com/briansmith/ring |
| rustc_version | 0.4.1 | MIT OR Apache-2.0 | https://github.com/djc/rustc-version-rs |
| rustls | 0.23.46 | Apache-2.0 OR ISC OR MIT | https://github.com/rustls/rustls |
| rustls-native-certs | 0.8.5 | Apache-2.0 OR ISC OR MIT | https://github.com/rustls/rustls-native-certs |
| rustls-pki-types | 1.16.0 | MIT OR Apache-2.0 | https://github.com/rustls/pki-types |
| rustls-platform-verifier | 0.7.1 | MIT OR Apache-2.0 | https://github.com/rustls/rustls-platform-verifier |
| rustls-platform-verifier-android | 0.2.0 | MIT OR Apache-2.0 | https://github.com/rustls/rustls-platform-verifier |
| rustls-webpki | 0.103.17 | ISC | https://github.com/rustls/webpki |
| same-file | 1.0.6 | Unlicense/MIT | https://github.com/BurntSushi/same-file |
| schannel | 0.1.30 | MIT | https://github.com/steffengy/schannel-rs |
| security-framework | 3.7.0 | MIT OR Apache-2.0 | https://github.com/kornelski/rust-security-framework |
| security-framework-sys | 2.17.0 | MIT OR Apache-2.0 | https://github.com/kornelski/rust-security-framework |
| semver | 1.0.28 | MIT OR Apache-2.0 | https://github.com/dtolnay/semver |
| serde | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes |
| shlex | 2.0.1 | MIT OR Apache-2.0 | https://github.com/comex/rust-shlex |
| simd_cesu8 | 1.2.0 | Apache-2.0 OR MIT | https://github.com/seancroach/simd_cesu8 |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 | https://github.com/rusticstuff/simdutf8 |
| subtle | 2.6.1 | BSD-3-Clause | https://github.com/dalek-cryptography/subtle |
| syn | 2.0.119 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| syn | 3.0.9 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| toml_datetime | 0.7.5+spec-1.1.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| toml_edit | 0.23.10+spec-1.0.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| toml_parser | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| toml_writer | 1.2.0+spec-1.1.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| typenum | 1.20.1 | MIT OR Apache-2.0 | https://github.com/paholg/typenum |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | https://github.com/dtolnay/unicode-ident |
| untrusted | 0.9.0 | ISC | https://github.com/briansmith/untrusted |
| ureq | 3.5.2 | MIT OR Apache-2.0 | https://github.com/algesten/ureq |
| ureq-proto | 0.7.1 | MIT OR Apache-2.0 | https://github.com/algesten/ureq-proto |
| utf8-zero | 0.8.1 | MIT OR Apache-2.0 | https://github.com/algesten/utf8-zero |
| version_check | 0.9.5 | MIT/Apache-2.0 | https://github.com/SergioBenitez/version_check |
| walkdir | 2.5.0 | Unlicense/MIT | https://github.com/BurntSushi/walkdir |
| wasi | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wasi |
| webpki-root-certs | 1.0.12 | CDLA-Permissive-2.0 | https://github.com/rustls/webpki-roots |
| webpki-roots | 1.0.12 | CDLA-Permissive-2.0 | https://github.com/rustls/webpki-roots |
| winapi-util | 0.1.11 | Unlicense OR MIT | https://github.com/BurntSushi/winapi-util |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows_aarch64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows_aarch64_msvc | 0.52.6 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows_i686_gnu | 0.52.6 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows_i686_gnullvm | 0.52.6 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows_i686_msvc | 0.52.6 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows_x86_64_gnu | 0.52.6 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows_x86_64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| winnow | 0.7.15 | MIT | https://github.com/winnow-rs/winnow |
| winnow | 1.0.5 | MIT | https://github.com/winnow-rs/winnow |
| zeroize | 1.9.2 | Apache-2.0 OR MIT | https://github.com/RustCrypto/utils |
| zmij | 1.0.23 | MIT | https://github.com/dtolnay/zmij |

## 清单维护

依赖或版本变化时重新核对并更新本文件：

```sh
cargo metadata --locked --format-version 1 \
  | jq -r '.packages[] | [.name, .version, .license, (.repository // "-")] | @tsv'
```

本清单于 2026-11-18 按上述命令与 `Cargo.lock` 核对生成。
